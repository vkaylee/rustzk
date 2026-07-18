//! Shared test utilities for rustzk integration tests.
//!
//! Provides a reusable [`MockZKServer`] builder that eliminates the need to
//! copy-paste TCP mock-server boilerplate across test files.

use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// A type-erased command handler: receives (reply_id, payload) and returns
/// an optional response as (response_command, response_payload).
pub type CmdHandler = dyn Fn(u16, &[u8]) -> Option<(u16, Vec<u8>)> + Send + 'static;

/// Builder for a TCP mock ZK device server.
///
/// # Quick start
///
/// ```ignore
/// let (server, port) = MockZKServer::new()
///     .on(CMD_CONNECT, |rid, _| Some((CMD_ACK_OK, vec![])))
///     .on(CMD_GET_TIME, |rid, _| {
///         let t = 839845230u32;
///         Some((CMD_ACK_OK, t.to_le_bytes().to_vec()))
///     })
///     .spawn();
/// ```
///
/// Unhandled commands receive `CMD_ACK_OK` with an empty payload by default.
/// Call [`no_default`] to disable this behaviour (connection errors out).
pub struct MockZKServer {
    session_id: u16,
    handlers: Vec<(u16, Box<CmdHandler>)>,
    default_ack: bool,
}

/// Handle to a spawned mock server — joins the server thread on drop.
pub struct MockServerHandle {
    handle: Option<thread::JoinHandle<()>>,
}

impl MockZKServer {
    /// Create a new builder with session ID 1234 and ACK_OK default responses.
    pub fn new() -> Self {
        MockZKServer {
            session_id: 1234,
            handlers: Vec::new(),
            default_ack: true,
        }
    }

    /// Set the session ID returned in response packets.
    pub fn with_session(mut self, id: u16) -> Self {
        self.session_id = id;
        self
    }

    /// Register a handler for a specific command code.
    ///
    /// The handler receives `(reply_id, payload)` and may return
    /// `Some((response_command, response_data))` or `None` to skip
    /// (falling through to the default).
    pub fn on(
        mut self,
        cmd: u16,
        handler: impl Fn(u16, &[u8]) -> Option<(u16, Vec<u8>)> + Send + 'static,
    ) -> Self {
        self.handlers.push((cmd, Box::new(handler)));
        self
    }

    /// Shorthand: respond with `CMD_ACK_OK` + given payload for a command.
    pub fn on_ack(mut self, cmd: u16, payload: Vec<u8>) -> Self {
        self.handlers.push((
            cmd,
            Box::new(move |_rid, _p| Some((CMD_ACK_OK, payload.clone()))),
        ));
        self
    }

    /// Disable the default `CMD_ACK_OK` response for unhandled commands.
    /// Unhandled commands will cause the server to panic (fail loudly in tests).
    pub fn no_default(mut self) -> Self {
        self.default_ack = false;
        self
    }

    /// Spawn the mock server on a random port and return the handle + port.
    pub fn spawn(self) -> (MockServerHandle, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("MockZKServer: bind failed");
        let port = listener.local_addr().unwrap().port();
        let session_id = self.session_id;
        let handlers = self.handlers;
        let default_ack = self.default_ack;

        let handle = thread::spawn(move || {
            let (mut stream, _) = match listener.accept() {
                Ok(c) => c,
                Err(_) => return,
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .ok();

            loop {
                // ── read TCP header ──
                let mut header = [0u8; 8];
                if stream.read_exact(&mut header).is_err() {
                    break;
                }
                let (length, _) = match TCPWrapper::decode_header(&header) {
                    Ok(v) => v,
                    Err(_) => break,
                };
                let mut body = vec![0u8; length];
                if stream.read_exact(&mut body).is_err() {
                    break;
                }
                let packet = match ZKPacket::from_bytes_owned(body) {
                    Ok(p) => p,
                    Err(_) => break,
                };

                let cmd = packet.command();
                let rid = packet.reply_id();
                let payload = packet.into_payload().into_owned();

                // ── EXIT always handled first ──
                if cmd == CMD_EXIT {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, rid, vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                    break;
                }

                // ── dispatch to registered handlers ──
                let mut responded = false;
                for (h_cmd, handler) in &handlers {
                    if *h_cmd == cmd {
                        if let Some((resp_cmd, resp_data)) = handler(rid, &payload) {
                            let res = ZKPacket::new(resp_cmd, session_id, rid, resp_data);
                            let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                            responded = true;
                            break;
                        }
                    }
                }

                if !responded {
                    if default_ack {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, rid, vec![]);
                        let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                    } else {
                        panic!(
                            "MockZKServer: no handler for command 0x{:X} ({}), and default_ack disabled",
                            cmd, cmd
                        );
                    }
                }
            }
        });

        (
            MockServerHandle {
                handle: Some(handle),
            },
            port,
        )
    }
}

impl Default for MockZKServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockServerHandle {
    /// Wait for the server thread to finish.
    pub fn join(mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for MockServerHandle {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Convenience: create a simple read_sizes payload (20 × i32).
/// Index mapping follows the pyzk convention:
///   4=users, 8=records, 15=users_cap, 16=rec_cap
pub fn make_sizes_payload(users: i32, records: i32, users_cap: i32, rec_cap: i32) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut bytes = Vec::with_capacity(80);
    for i in 0..20 {
        let val: i32 = match i {
            4 => users,
            8 => records,
            15 => users_cap,
            16 => rec_cap,
            _ => 0,
        };
        bytes.write_i32::<LittleEndian>(val).unwrap();
    }
    bytes
}

/// Convenience: make a 28-byte user payload for `CMD_DATA` responses.
pub fn make_small_user_data(uid: u16, privilege: u8, name: &[u8; 8], user_id: u32) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut data = Vec::with_capacity(32);
    data.write_u32::<LittleEndian>(28).unwrap(); // size prefix
    data.write_u16::<LittleEndian>(uid).unwrap();
    data.push(privilege);
    data.extend_from_slice(b"\0\0\0\0\0"); // password (5)
    data.extend_from_slice(name); // name (8)
    data.write_u32::<LittleEndian>(0).unwrap(); // card
    data.push(0); // pad
    data.push(1); // group
    data.write_u16::<LittleEndian>(0).unwrap(); // tz
    data.write_u32::<LittleEndian>(user_id).unwrap();
    data
}

/// Convenience: build an 8-byte attendance record.
pub fn make_attendance_record(uid: u16, status: u8, time: u32, punch: u8) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut data = Vec::with_capacity(12);
    data.write_u32::<LittleEndian>(8).unwrap(); // size prefix
    data.write_u16::<LittleEndian>(uid).unwrap();
    data.push(status);
    data.write_u32::<LittleEndian>(time).unwrap();
    data.push(punch);
    data
}

/// Convenience: build a PREPARE_DATA response payload.
pub fn make_prepare_data_payload(size: u32) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut payload = vec![0u8; 5];
    payload[0] = 1;
    (&mut payload[1..5])
        .write_u32::<LittleEndian>(size)
        .unwrap();
    payload
}

/// State wrapper for handlers that need mutable state.
/// Use with `Arc::new(Mutex::new(...))` when building a handler closure.
///
/// # Example
/// ```ignore
/// let last_cmd = Arc::new(Mutex::new(0u16));
/// let last = last_cmd.clone();
/// server = server.on(_CMD_PREPARE_BUFFER, move |rid, payload| {
///     let cmd_in_buf = LittleEndian::read_u16(&payload[1..3]);
///     *last.lock().unwrap() = cmd_in_buf;
///     Some((CMD_PREPARE_DATA, make_prepare_data_payload(32)))
/// });
/// ```
pub fn with_state<T: Send + 'static>(init: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(init))
}

// ── Lightweight helpers for inline mock servers ──────────────────────────

/// Read one TCP-framed ZK request from the client stream.
/// Panics on I/O or decode errors (acceptable in test code).
pub fn read_request(stream: &mut TcpStream) -> ZKPacket<'static> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).unwrap();
    let (length, _) = TCPWrapper::decode_header(&header).unwrap();
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).unwrap();
    ZKPacket::from_bytes_owned(body).unwrap()
}

/// Read one TCP-framed ZK request, returning `None` on clean EOF / error.
pub fn try_read_request(stream: &mut TcpStream) -> Option<ZKPacket<'static>> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).ok()?;
    let (length, _) = TCPWrapper::decode_header(&header).ok()?;
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).ok()?;
    ZKPacket::from_bytes_owned(body).ok()
}

/// Send a TCP-framed ZK response to the client stream.
pub fn send_response(stream: &mut TcpStream, packet: &ZKPacket) {
    stream
        .write_all(&TCPWrapper::wrap(&packet.to_bytes()))
        .unwrap();
}
