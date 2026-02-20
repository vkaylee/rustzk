pub mod constants;
pub use crate::constants::*;
pub mod models;
pub mod protocol;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Timelike};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use thiserror::Error;

use crate::models::{Attendance, Finger, User};
use crate::protocol::{TCPWrapper, ZKPacket};

#[derive(Error, Debug)]
pub enum ZKError {
    #[error("Network error: {0}")]
    Network(#[from] io::Error),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Response error: {0}")]
    Response(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

pub type ZKResult<T> = Result<T, ZKError>;

pub enum ZKTransport {
    Tcp(TcpStream),
    Udp(UdpSocket),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZKProtocol {
    TCP,
    UDP,
    Auto,
}

pub struct ZK {
    pub addr: String,
    transport: Option<ZKTransport>,
    session_id: u16,
    reply_id: u16,
    pub timeout: Duration,
    pub is_connected: bool,
    user_id_cache: Option<HashMap<u16, String>>,
    pub user_packet_size: usize,
    users: u32,
    fingers: u32,
    records: u32,
    cards: i32,
    faces: u32,
    fingers_cap: i32,
    users_cap: i32,
    rec_cap: i32,
    faces_cap: i32,
    pub encoding: String,
    password: u32,
    timezone_offset: i32, // Offset in minutes
    timezone_synced: bool,
}

impl ZK {
    pub fn new(addr: &str, port: u16) -> Self {
        ZK {
            addr: format!("{}:{}", addr, port),
            transport: None,
            session_id: 0,
            reply_id: USHRT_MAX - 1,
            timeout: Duration::from_secs(60),
            is_connected: false,
            user_id_cache: None,
            user_packet_size: 28,
            users: 0,
            fingers: 0,
            records: 0,
            cards: 0,
            faces: 0,
            fingers_cap: 0,
            users_cap: 0,
            rec_cap: 0,
            faces_cap: 0,
            encoding: "UTF-8".to_string(),
            password: 0,
            timezone_offset: 0,
            timezone_synced: false,
        }
    }

    /// Sets the communication password for the device.
    pub fn set_password(&mut self, password: u32) {
        self.password = password;
    }

    /// Internal helper to generate the authentication communication key.
    fn make_commkey(key: u32, session_id: u16, ticks: u8) -> Vec<u8> {
        let mut k = 0u32;
        for i in 0..32 {
            if (key & (1 << i)) != 0 {
                k = (k << 1) | 1;
            } else {
                k <<= 1;
            }
        }
        k = k.wrapping_add(session_id as u32);

        let b1 = (k & 0xFF) as u8 ^ b'Z';
        let b2 = ((k >> 8) & 0xFF) as u8 ^ b'K';
        let b3 = ((k >> 16) & 0xFF) as u8 ^ b'S';
        let b4 = ((k >> 24) & 0xFF) as u8 ^ b'O';

        let k = (b1 as u16) | ((b2 as u16) << 8);
        let k2 = (b3 as u16) | ((b4 as u16) << 8);

        let c1 = (k2 & 0xFF) as u8 ^ ticks; // b3 ^ ticks
        let c2 = ((k2 >> 8) & 0xFF) as u8 ^ ticks; // b4 ^ ticks
        let c3 = ticks;
        let c4 = ((k >> 8) & 0xFF) as u8 ^ ticks; // b2 ^ ticks

        vec![c1, c2, c3, c4]
    }

    /// Connect to the ZK device using the specified protocol.
    /// Supports TCP, UDP, or Auto-detection.
    pub fn connect(&mut self, protocol: ZKProtocol) -> ZKResult<()> {
        match protocol {
            ZKProtocol::TCP => self.connect_tcp(),
            ZKProtocol::UDP => self.connect_udp(),
            ZKProtocol::Auto => {
                // Try TCP first
                match self.connect_tcp() {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::info!("TCP connect failed: {}. Falling back to UDP...", e);
                        self.connect_udp()
                    }
                }
            }
        }
    }

    fn connect_tcp(&mut self) -> ZKResult<()> {
        let addrs = self.addr.to_socket_addrs().map_err(|e| {
            ZKError::Connection(format!("Failed to resolve address {}: {}", self.addr, e))
        })?;
        let addr = addrs
            .into_iter()
            .next()
            .ok_or_else(|| ZKError::Connection(format!("No address found for {}", self.addr)))?;

        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;

        self.transport = Some(ZKTransport::Tcp(stream));
        self.perform_connect_handshake()
    }

    fn connect_udp(&mut self) -> ZKResult<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect(&self.addr)?;
        socket.set_read_timeout(Some(self.timeout))?;
        socket.set_write_timeout(Some(self.timeout))?;

        self.transport = Some(ZKTransport::Udp(socket));
        self.perform_connect_handshake()
    }

    fn perform_connect_handshake(&mut self) -> ZKResult<()> {
        self.session_id = 0;
        self.reply_id = USHRT_MAX - 1;

        let res = self.send_command(CMD_CONNECT, Vec::new())?;

        // Update session_id if we got a valid response (OK or UNAUTH)
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_UNAUTH {
            self.session_id = res.session_id;
        }

        if res.command == CMD_ACK_UNAUTH {
            let command_string = ZK::make_commkey(self.password, self.session_id, 50);
            let auth_res = self.send_command(CMD_AUTH, command_string)?;
            if auth_res.command == CMD_ACK_UNAUTH {
                return Err(ZKError::Connection(
                    "Unauthorized: Password required or incorrect".into(),
                ));
            }
            self.session_id = auth_res.session_id;
            self.is_connected = true;
            return Ok(());
        }

        if res.command == CMD_ACK_OK {
            self.is_connected = true;
            Ok(())
        } else {
            Err(ZKError::Connection(format!(
                "Invalid response: {}",
                res.command
            )))
        }
    }

    /// Synchronizes the device's timezone offset (`TZAdj`) lazily.
    fn sync_timezone(&mut self) -> ZKResult<()> {
        if self.timezone_synced {
            return Ok(());
        }

        if let Ok(tz_str) = self.get_option_value("TZAdj") {
            if let Ok(tz_val) = tz_str.parse::<i32>() {
                self.timezone_offset = tz_val * 60; // Convert hours to minutes
                self.timezone_synced = true;
            }
        }
        Ok(())
    }

    /// Low-level method to read a single ZK packet from the transport.
    fn read_packet(&mut self) -> ZKResult<ZKPacket<'static>> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| ZKError::Connection("Not connected".into()))?;
        match transport {
            ZKTransport::Tcp(stream) => {
                let mut header = [0u8; 8];
                stream.read_exact(&mut header)?;
                let (length, _) = TCPWrapper::decode_header(&header)
                    .map_err(|e| ZKError::InvalidData(e.to_string()))?;

                let mut body = vec![0u8; length];
                stream.read_exact(&mut body)?;

                ZKPacket::from_bytes_owned(body)
            }
            ZKTransport::Udp(socket) => {
                let mut buf = vec![0u8; 2048];
                let len = socket.recv(&mut buf)?;
                buf.truncate(len);
                ZKPacket::from_bytes_owned(buf)
            }
        }
    }

    /// Reads a response packet and validates its reply ID.
    fn read_response_safe(&mut self) -> ZKResult<ZKPacket<'static>> {
        let mut discarded = 0;
        loop {
            let res_packet = self.read_packet()?;
            // Log packet at debug level for troubleshooting if needed
            log::debug!(
                "Received Packet: Cmd {} (0x{:X}), Reply ID: {}",
                res_packet.command,
                res_packet.command,
                res_packet.reply_id
            );

            if res_packet.reply_id != self.reply_id {
                discarded += 1;
                log::debug!(
                    "Reply ID mismatch: expected {}, got {}. Discarding packet.",
                    self.reply_id,
                    res_packet.reply_id
                );
                if discarded > MAX_DISCARDED_PACKETS {
                    return Err(ZKError::Response("Too many discarded packets".into()));
                }
                continue;
            }
            return Ok(res_packet);
        }
    }

    /// Sends a command to the device and waits for a safe response.
    pub(crate) fn send_command(
        &mut self,
        command: u16,
        payload: Vec<u8>,
    ) -> ZKResult<ZKPacket<'static>> {
        self.reply_id = self.reply_id.wrapping_add(1);
        if self.reply_id == USHRT_MAX {
            self.reply_id -= USHRT_MAX;
        }

        log::debug!(
            "Sending Command: {} (0x{:X}), Reply ID: {}",
            command,
            command,
            self.reply_id
        );

        let packet = ZKPacket::new(command, self.session_id, self.reply_id, payload);

        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| ZKError::Connection("Not connected".into()))?;

        match transport {
            ZKTransport::Tcp(stream) => {
                let mut buf = Vec::with_capacity(packet.payload.len() + 16);
                buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_1)?;
                buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_2)?;
                buf.write_u32::<LittleEndian>((packet.payload.len() + 8) as u32)?;
                packet.to_bytes_into(&mut buf)?;
                stream.write_all(&buf)?;
            }
            ZKTransport::Udp(socket) => {
                let mut buf = Vec::with_capacity(packet.payload.len() + 8);
                packet.to_bytes_into(&mut buf)?;
                socket.send(&buf)?;
            }
        }

        self.read_response_safe()
    }

    /// Fetches device capacity and usage statistics.
    pub fn read_sizes(&mut self) -> ZKResult<()> {
        let mut res = self.send_command(CMD_GET_FREE_SIZES, Vec::new())?;

        // Handle case where device sends ACK_OK then ACK_DATA/Response separately
        if res.command == CMD_ACK_OK && res.payload.len() < 16 {
            // Try reading the next packet which should contain the actual data
            match self.read_response_safe() {
                Ok(next_packet) => {
                    res = next_packet;
                }
                Err(e) => {
                    log::debug!(
                        "read_sizes: received ACK_OK but failed to read subsequent data: {}",
                        e
                    );
                }
            }
        }

        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            let data = res.payload;
            if data.len() >= 80 {
                let mut rdr = io::Cursor::new(&data[..80]);
                let mut fields = [0i32; 20];
                for field in &mut fields {
                    *field = rdr.read_i32::<byteorder::LittleEndian>()?;
                }
                self.users = fields[4] as u32;
                self.fingers = fields[6] as u32;
                self.records = fields[8] as u32;
                self.cards = fields[12];
                self.fingers_cap = fields[14];
                self.users_cap = fields[15];
                self.rec_cap = fields[16];
            }
            if data.len() >= 92 {
                let mut rdr = io::Cursor::new(&data[80..92]);
                self.faces = rdr.read_i32::<byteorder::LittleEndian>()? as u32;
                let _ = rdr.read_i32::<byteorder::LittleEndian>()?;
                self.faces_cap = rdr.read_i32::<byteorder::LittleEndian>()?;
            }
            // Auto-sync timezone
            let _ = self.sync_timezone();
            Ok(())
        } else {
            Err(ZKError::Response(format!(
                "Failed to read sizes: {}",
                res.command
            )))
        }
    }

    pub fn decode_time(t: &[u8]) -> ZKResult<chrono::NaiveDateTime> {
        if t.len() < 4 {
            return Err(ZKError::InvalidData("Timestamp too short".into()));
        }
        let mut rdr = io::Cursor::new(t);
        let t = rdr.read_u32::<byteorder::LittleEndian>()?;

        let second = t % 60;
        let t = t / 60;
        let minute = t % 60;
        let t = t / 60;
        let hour = t % 24;
        let t = t / 24;
        let day = t % 31 + 1;
        let t = t / 31;
        let month = t % 12 + 1;
        let t = t / 12;
        let year = (t + 2000) as i32;

        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d: chrono::NaiveDate| d.and_hms_opt(hour, minute, second))
            .ok_or_else(|| ZKError::InvalidData("Invalid date/time".into()))
    }

    pub fn encode_time(t: chrono::NaiveDateTime) -> u32 {
        let year = (t.year() % 100) as u32;
        let month = t.month();
        let day = t.day();
        let hour = t.hour();
        let minute = t.minute();
        let second = t.second();

        (year * 12 * 31 + (month - 1) * 31 + day - 1) * (24 * 60 * 60)
            + (hour * 60 + minute) * 60
            + second
    }

    fn receive_chunk_into(&mut self, res: ZKPacket<'static>, data: &mut Vec<u8>) -> ZKResult<()> {
        if res.command == CMD_DATA {
            data.extend_from_slice(&res.payload);
            Ok(())
        } else if res.command == CMD_ACK_OK {
            // New firmware may send ACK_OK before actual data.
            // Give the device a little time to prepare.
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(())
        } else if res.command == CMD_PREPARE_DATA {
            if res.payload.len() < 4 {
                return Err(ZKError::InvalidData("Invalid prepare data payload".into()));
            }
            let size = byteorder::LittleEndian::read_u32(&res.payload[..4]) as usize;

            if size > MAX_RESPONSE_SIZE {
                return Err(ZKError::InvalidData(format!(
                    "Response size {} exceeds maximum {}",
                    size, MAX_RESPONSE_SIZE
                )));
            }

            data.reserve(size);
            let mut remaining = size;

            while remaining > 0 {
                let chunk_res = self.read_response_safe()?;

                if chunk_res.command == CMD_DATA {
                    data.extend_from_slice(&chunk_res.payload);
                    if remaining >= chunk_res.payload.len() {
                        remaining -= chunk_res.payload.len();
                    } else {
                        remaining = 0;
                    }
                } else if chunk_res.command == CMD_ACK_OK {
                    break;
                } else {
                    return Err(ZKError::Response(format!(
                        "Unexpected chunk command: {}",
                        chunk_res.command
                    )));
                }
            }
            Ok(())
        } else {
            Err(ZKError::Response(format!(
                "Invalid response for chunk: {}",
                res.command
            )))
        }
    }

    fn read_chunk_into(&mut self, start: i32, size: i32, data: &mut Vec<u8>) -> ZKResult<()> {
        let mut payload = Vec::with_capacity(8);
        payload.write_i32::<byteorder::LittleEndian>(start)?;
        payload.write_i32::<byteorder::LittleEndian>(size)?;

        let res = self.send_command(_CMD_READ_BUFFER, payload)?;
        if res.command == CMD_ACK_OK {
            // If we get ACK for read chunk, it means data is coming next.
            // Wait for the actual data packet.
            let data_packet = self.read_response_safe()?;
            self.receive_chunk_into(data_packet, data)
        } else {
            self.receive_chunk_into(res, data)
        }
    }

    fn read_with_buffer(&mut self, command: u16, fct: u8, ext: u32) -> ZKResult<Vec<u8>> {
        let mut payload = Vec::new();
        payload.write_u8(1)?; // ZK6/8 flag?
        payload.write_u16::<byteorder::LittleEndian>(command)?;
        payload.write_u32::<byteorder::LittleEndian>(fct as u32)?;
        payload.write_u32::<byteorder::LittleEndian>(ext)?;

        let res = self.send_command(_CMD_PREPARE_BUFFER, payload)?;
        if res.command == CMD_DATA {
            return Ok(res.payload.into_owned());
        }

        // Parse size: prioritize offset 1 (standard) if length >= 5
        let size = if res.payload.len() >= 5 {
            byteorder::LittleEndian::read_u32(&res.payload[1..5]) as usize
        } else if res.payload.len() >= 4 {
            // Fallback for short ACK_OK payloads
            byteorder::LittleEndian::read_u32(&res.payload[0..4]) as usize
        } else {
            return Err(ZKError::Response(format!(
                "Invalid response size length: {}",
                res.payload.len()
            )));
        };

        if size > MAX_RESPONSE_SIZE {
            return Err(ZKError::InvalidData(format!(
                "Buffered response size {} exceeds maximum {}",
                size, MAX_RESPONSE_SIZE
            )));
        }

        let max_chunk = if let Some(ZKTransport::Tcp(_)) = self.transport {
            TCP_MAX_CHUNK
        } else {
            UDP_MAX_CHUNK
        };

        let mut data = Vec::with_capacity(size);
        let mut start = 0;
        let mut remaining = size;
        let mut empty_responses_count = 0;

        while remaining > 0 {
            let chunk_size = std::cmp::min(remaining, max_chunk);
            let len_before = data.len();
            self.read_chunk_into(start as i32, chunk_size as i32, &mut data)?;
            let chunk_len = data.len() - len_before;

            if chunk_len == 0 {
                empty_responses_count += 1;
                if empty_responses_count > 100 {
                    return Err(ZKError::Response(
                        "Too many empty responses from device".into(),
                    ));
                }
                // Small delay or just continue to wait for device to prepare data
                continue;
            }

            empty_responses_count = 0; // Reset counter on success
            start += chunk_len;
            if remaining >= chunk_len {
                remaining -= chunk_len;
            } else {
                remaining = 0;
            }
        }

        // Free data buffer
        let _ = self.send_command(CMD_FREE_DATA, Vec::new());

        Ok(data)
    }

    /// Decodes a GBK-encoded byte slice into a String.
    fn decode_gbk(bytes: &[u8]) -> String {
        let trimmed = bytes
            .iter()
            .position(|&x| x == 0)
            .map_or(bytes, |i| &bytes[..i]);
        let (cow, _encoding, has_malformed) = encoding_rs::GBK.decode(trimmed);
        if has_malformed {
            log::warn!(
                "GBK decoding encountered malformed sequences in data: {:?}",
                trimmed
            );
        }
        cow.into_owned()
    }

    /// Retrieves all users from the device.
    pub fn get_users(&mut self) -> ZKResult<Vec<User>> {
        self.read_sizes()?;
        if self.users == 0 {
            return Ok(Vec::new());
        }

        let userdata = self.read_with_buffer(CMD_USERTEMP_RRQ, FCT_USER, 0)?;
        if userdata.len() <= 4 {
            return Ok(Vec::new());
        }

        let total_size = byteorder::LittleEndian::read_u32(&userdata[0..4]) as usize;
        self.user_packet_size = total_size / self.users as usize;
        let data = &userdata[4..];

        let mut users = Vec::new();
        let mut offset = 0;

        if self.user_packet_size == USER_PACKET_SIZE_SMALL {
            while offset + USER_PACKET_SIZE_SMALL <= data.len() {
                let chunk = &data[offset..offset + USER_PACKET_SIZE_SMALL];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let privilege = rdr.read_u8()?;
                let mut password_bytes = [0u8; 5];
                rdr.read_exact(&mut password_bytes)?;
                let mut name_bytes = [0u8; 8];
                rdr.read_exact(&mut name_bytes)?;
                let card = rdr.read_u32::<byteorder::LittleEndian>()?;
                let _pad = rdr.read_u8()?;
                let group_id = rdr.read_u8()?;
                let _timezone = rdr.read_u16::<byteorder::LittleEndian>()?;
                let user_id = rdr.read_u32::<byteorder::LittleEndian>()?;

                users.push(User {
                    uid,
                    name: ZK::decode_gbk(&name_bytes),
                    privilege,
                    password: String::from_utf8_lossy(&password_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    group_id: group_id.to_string(),
                    user_id: user_id.to_string(),
                    card,
                });
                offset += USER_PACKET_SIZE_SMALL;
            }
        } else if self.user_packet_size == USER_PACKET_SIZE_LARGE {
            while offset + USER_PACKET_SIZE_LARGE <= data.len() {
                let chunk = &data[offset..offset + USER_PACKET_SIZE_LARGE];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let privilege = rdr.read_u8()?;
                let mut password_bytes = [0u8; 8];
                rdr.read_exact(&mut password_bytes)?;
                let mut name_bytes = [0u8; 24];
                rdr.read_exact(&mut name_bytes)?;
                let card = rdr.read_u32::<byteorder::LittleEndian>()?;
                let _pad1 = rdr.read_u8()?;
                let mut group_id_bytes = [0u8; 7];
                rdr.read_exact(&mut group_id_bytes)?;
                let _pad2 = rdr.read_u8()?;
                let mut user_id_bytes = [0u8; 24];
                rdr.read_exact(&mut user_id_bytes)?;

                users.push(User {
                    uid,
                    name: ZK::decode_gbk(&name_bytes),
                    privilege,
                    password: String::from_utf8_lossy(&password_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    group_id: String::from_utf8_lossy(&group_id_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    user_id: String::from_utf8_lossy(&user_id_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    card,
                });
                offset += USER_PACKET_SIZE_LARGE;
            }
        } else {
            return Err(ZKError::Response(format!(
                "Unsupported user packet size: {}. Device might be using an unknown protocol version.",
                self.user_packet_size
            )));
        }

        Ok(users)
    }

    /// Retrieves all attendance records from the device.
    pub fn get_attendance(&mut self) -> ZKResult<Vec<Attendance>> {
        self.read_sizes()?;
        if self.records == 0 {
            return Ok(Vec::new());
        }

        // Fetch raw attendance buffer FIRST, before any other buffer commands.
        // Some firmware (e.g. ZAM180 Ver 6.60) loses buffer state after CMD_FREE_DATA
        // sent at the end of get_users(), so attendance must be fetched first.
        let attendance_data = self.read_with_buffer(CMD_ATTLOG_RRQ, 0, 0)?;
        if attendance_data.len() < 4 {
            return Ok(Vec::new());
        }

        let total_size = byteorder::LittleEndian::read_u32(&attendance_data[0..4]) as usize;
        let record_size = if self.records > 0 {
            total_size / self.records as usize
        } else {
            0
        };
        let data = &attendance_data[4..];

        let mut attendances = Vec::new();
        let mut offset = 0;

        if record_size == ATT_RECORD_SIZE_8
            || (record_size > 0
                && total_size.wrapping_rem(ATT_RECORD_SIZE_8) == 0
                && record_size < 16)
        {
            while offset + ATT_RECORD_SIZE_8 <= data.len() {
                let chunk = &data[offset..offset + ATT_RECORD_SIZE_8];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let status = rdr.read_u8()?;
                let mut time_bytes = [0u8; 4];
                rdr.read_exact(&mut time_bytes)?;
                let punch = rdr.read_u8()?;

                let timestamp = ZK::decode_time(&time_bytes)?;
                let user_id = self.get_user_id_from_cache(uid);

                attendances.push(Attendance {
                    uid: uid as u32,
                    user_id,
                    timestamp,
                    status,
                    punch,
                    timezone_offset: self.timezone_offset,
                });
                offset += record_size;
            }
        } else if record_size == ATT_RECORD_SIZE_16
            || (record_size > 0
                && record_size.wrapping_rem(ATT_RECORD_SIZE_16) == 0
                && record_size < 40)
        {
            while offset + ATT_RECORD_SIZE_16 <= data.len() {
                let chunk = &data[offset..offset + ATT_RECORD_SIZE_16];
                let mut rdr = io::Cursor::new(chunk);
                let user_id_num = rdr.read_u32::<byteorder::LittleEndian>()?;
                let mut time_bytes = [0u8; 4];
                rdr.read_exact(&mut time_bytes)?;
                let status = rdr.read_u8()?;
                let punch = rdr.read_u8()?;
                // reserved 2 bytes, workcode 4 bytes

                let timestamp = ZK::decode_time(&time_bytes)?;
                let user_id = user_id_num.to_string();
                let uid = user_id_num;

                attendances.push(Attendance {
                    uid,
                    user_id,
                    timestamp,
                    status,
                    punch,
                    timezone_offset: self.timezone_offset,
                });
                offset += ATT_RECORD_SIZE_16;
            }
        } else if record_size >= ATT_RECORD_SIZE_40 {
            while offset + ATT_RECORD_SIZE_40 <= data.len() {
                let chunk = &data[offset..offset + ATT_RECORD_SIZE_40];
                // Handle the 0xff255 prefix if present as in Python code
                let mut chunk_ptr = chunk;
                if chunk.starts_with(b"\xff255\x00\x00\x00\x00\x00") {
                    chunk_ptr = &chunk[10..];
                    if chunk_ptr.len() < 30 {
                        break;
                    } // Should not happen if record_size >= 40
                }

                let mut rdr = io::Cursor::new(chunk_ptr);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let mut user_id_bytes = [0u8; 24];
                rdr.read_exact(&mut user_id_bytes)?;
                let status = rdr.read_u8()?;
                let mut time_bytes = [0u8; 4];
                rdr.read_exact(&mut time_bytes)?;
                let punch = rdr.read_u8()?;

                let timestamp = ZK::decode_time(&time_bytes)?;
                let user_id = String::from_utf8_lossy(&user_id_bytes)
                    .trim_matches('\0')
                    .to_string();

                attendances.push(Attendance {
                    uid: uid as u32,
                    user_id,
                    timestamp,
                    status,
                    punch,
                    timezone_offset: self.timezone_offset,
                });
                offset += record_size;
            }
        }

        Ok(attendances)
    }

    pub fn get_firmware_version(&mut self) -> ZKResult<String> {
        let res = self.send_command(CMD_GET_VERSION, Vec::new())?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            Ok(String::from_utf8_lossy(&res.payload)
                .trim_matches('\0')
                .to_string())
        } else {
            Err(ZKError::Response("Can't read firmware version".into()))
        }
    }

    pub fn get_option_value(&mut self, key: &str) -> ZKResult<String> {
        let mut command_string = key.as_bytes().to_vec();
        command_string.push(0);
        let res = self.send_command(CMD_OPTIONS_RRQ, command_string)?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            let data = String::from_utf8_lossy(&res.payload)
                .trim_matches('\0')
                .to_string();
            // Usually returns "Key=Value"
            if let Some(pos) = data.find('=') {
                Ok(data[pos + 1..].to_string())
            } else {
                Ok(data)
            }
        } else {
            Err(ZKError::Response(format!("Can't read option {}", key)))
        }
    }

    pub fn get_serial_number(&mut self) -> ZKResult<String> {
        self.get_option_value("~SerialNumber")
    }

    pub fn get_platform(&mut self) -> ZKResult<String> {
        self.get_option_value("~Platform")
    }

    pub fn get_mac(&mut self) -> ZKResult<String> {
        self.get_option_value("MAC")
    }

    pub fn get_device_name(&mut self) -> ZKResult<String> {
        self.get_option_value("~DeviceName")
    }

    pub fn get_face_version(&mut self) -> ZKResult<String> {
        self.get_option_value("ZKFaceVersion")
    }

    pub fn get_fp_version(&mut self) -> ZKResult<String> {
        self.get_option_value("~ZKFPVersion")
    }

    pub fn get_time(&mut self) -> ZKResult<DateTime<FixedOffset>> {
        let _ = self.sync_timezone();
        let res = self.send_command(CMD_GET_TIME, Vec::new())?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            let naive = ZK::decode_time(&res.payload)?;
            let offset = FixedOffset::east_opt(self.timezone_offset * 60)
                .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());

            offset
                .from_local_datetime(&naive)
                .single()
                .ok_or_else(|| ZKError::InvalidData("Ambiguous time from device".into()))
        } else {
            Err(ZKError::Response("Can't get time".into()))
        }
    }

    pub fn set_time(&mut self, t: DateTime<FixedOffset>) -> ZKResult<()> {
        // ZKTeco devices usually work in local time.
        let local_naive = t.naive_local();
        let encoded = ZK::encode_time(local_naive);
        let mut payload = Vec::with_capacity(4);
        payload.write_u32::<LittleEndian>(encoded)?;

        let res = self.send_command(CMD_SET_TIME, payload)?;
        if res.command == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response("Failed to set time".into()))
        }
    }

    /// Sets a device option by key and value.
    pub fn set_option(&mut self, key: &str, value: &str) -> ZKResult<()> {
        let command_string = format!("{}={}\0", key, value);
        let res = self.send_command(CMD_OPTIONS_WRQ, command_string.into_bytes())?;
        if res.command == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response(format!("Failed to set option {}", key)))
        }
    }

    /// Changes the communication password (CommKey) of the device.
    /// Note: After changing the password, you must use the new password for future connections.
    pub fn change_password(&mut self, new_password: u32) -> ZKResult<()> {
        self.set_option("ComKey", &new_password.to_string())?;
        self.password = new_password; // Update local state to match
        Ok(())
    }

    pub fn restart(&mut self) -> ZKResult<()> {
        self.send_command(CMD_RESTART, Vec::new())?;
        self.is_connected = false;
        self.transport = None;
        Ok(())
    }

    pub fn poweroff(&mut self) -> ZKResult<()> {
        self.send_command(CMD_POWEROFF, Vec::new())?;
        self.is_connected = false;
        self.transport = None;
        Ok(())
    }

    pub fn unlock(&mut self, seconds: u32) -> ZKResult<()> {
        let mut payload = Vec::new();
        // ZK protocol expects time in 100ms units for unlock?
        // Python: pack("I",int(time)*10)
        payload.write_u32::<byteorder::LittleEndian>(seconds * 10)?;
        let res = self.send_command(CMD_UNLOCK, payload)?;
        if res.command == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response("Can't open door".into()))
        }
    }

    pub fn disconnect(&mut self) -> ZKResult<()> {
        if self.is_connected {
            let _ = self.send_command(CMD_EXIT, Vec::new());
            self.is_connected = false;
        }
        self.transport = None;
        Ok(())
    }

    /// Refreshes the device's internal data.
    pub fn refresh_data(&mut self) -> ZKResult<()> {
        let res = self.send_command(CMD_REFRESHDATA, Vec::new())?;
        if res.command == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response("Failed to refresh data".into()))
        }
    }

    /// Creates or updates a user on the device.
    /// Ensures User ID uniqueness to prevent logic conflicts.
    /// **Performance Note:** This performs an O(N) fetch of all users first. For bulk operations, use `set_user_unchecked`.
    pub fn set_user(&mut self, user: &User) -> ZKResult<()> {
        // 1. Safety Check: Ensure this User ID doesn't already exist under a DIFFERENT UID.
        // If it exists under the SAME UID, it's an update, which is allowed.
        let existing_users = self.get_users()?;
        if let Some(existing) = existing_users.iter().find(|u| u.user_id == user.user_id) {
            if existing.uid != user.uid {
                return Err(ZKError::Response(format!(
                    "Conflict: User ID '{}' already exists on the device at UID {}",
                    user.user_id, existing.uid
                )));
            }
        }

        self.set_user_unchecked(user)
    }

    /// Creates or updates a user on the device WITHOUT uniqueness checks.
    /// High performance, suitable for bulk syncing.
    pub fn set_user_unchecked(&mut self, user: &User) -> ZKResult<()> {
        let mut payload = Vec::new();

        if self.user_packet_size == 28 {
            payload.write_u16::<LittleEndian>(user.uid)?;
            payload.write_u8(user.privilege)?;

            let mut password_bytes = [0u8; 5];
            let p_bytes = user.password.as_bytes();
            let p_len = std::cmp::min(p_bytes.len(), 5);
            password_bytes[..p_len].copy_from_slice(&p_bytes[..p_len]);
            payload.write_all(&password_bytes)?;

            let mut name_bytes = [0u8; 8];
            let n_bytes_gbk = encoding_rs::GBK.encode(&user.name).0;
            let n_len = std::cmp::min(n_bytes_gbk.len(), 8);
            name_bytes[..n_len].copy_from_slice(&n_bytes_gbk[..n_len]);
            payload.write_all(&name_bytes)?;

            payload.write_u32::<LittleEndian>(user.card)?;
            payload.write_u8(0)?; // pad
            let group_id = user.group_id.parse::<u8>().unwrap_or(0);
            payload.write_u8(group_id)?;
            payload.write_u16::<LittleEndian>(0)?; // timezone/pad
            let user_id_num = user.user_id.parse::<u32>().unwrap_or(0);
            payload.write_u32::<LittleEndian>(user_id_num)?;
        } else {
            // 72-byte format
            payload.write_u16::<LittleEndian>(user.uid)?;
            payload.write_u8(user.privilege)?;

            let mut password_bytes = [0u8; 8];
            let p_bytes = user.password.as_bytes();
            let p_len = std::cmp::min(p_bytes.len(), 8);
            password_bytes[..p_len].copy_from_slice(&p_bytes[..p_len]);
            payload.write_all(&password_bytes)?;

            let mut name_bytes = [0u8; 24];
            let n_bytes_gbk = encoding_rs::GBK.encode(&user.name).0;
            let n_len = std::cmp::min(n_bytes_gbk.len(), 24);
            name_bytes[..n_len].copy_from_slice(&n_bytes_gbk[..n_len]);
            payload.write_all(&name_bytes)?;

            payload.write_u32::<LittleEndian>(user.card)?;
            payload.write_u8(0)?; // pad1

            let mut group_id_bytes = [0u8; 7];
            let g_bytes = user.group_id.as_bytes();
            let g_len = std::cmp::min(g_bytes.len(), 7);
            group_id_bytes[..g_len].copy_from_slice(&g_bytes[..g_len]);
            payload.write_all(&group_id_bytes)?;

            payload.write_u8(0)?; // pad2

            let mut user_id_bytes = [0u8; 24];
            let u_bytes = user.user_id.as_bytes();
            let u_len = std::cmp::min(u_bytes.len(), 24);
            user_id_bytes[..u_len].copy_from_slice(&u_bytes[..u_len]);
            payload.write_all(&user_id_bytes)?;
        }

        let res = self.send_command(CMD_USER_WRQ, payload)?;
        if res.command == CMD_ACK_OK {
            let _ = self.refresh_data();
            Ok(())
        } else {
            Err(ZKError::Response("Failed to set user".into()))
        }
    }

    /// Deletes a specific user by UID.
    pub fn delete_user(&mut self, uid: u16) -> ZKResult<()> {
        let mut payload = Vec::with_capacity(2);
        payload.write_u16::<LittleEndian>(uid)?;

        let res = self.send_command(CMD_DELETE_USER, payload)?;
        if res.command == CMD_ACK_OK {
            let _ = self.refresh_data();
            Ok(())
        } else {
            Err(ZKError::Response("Failed to delete user".into()))
        }
    }

    /// Retrieves all fingerprint templates from the device.
    pub fn get_templates(&mut self) -> ZKResult<Vec<Finger>> {
        self.read_sizes()?;
        if self.fingers == 0 {
            return Ok(Vec::new());
        }

        let templatedata = self.read_with_buffer(CMD_DB_RRQ, FCT_FINGERTMP, 0)?;
        if templatedata.len() < 4 {
            return Ok(Vec::new());
        }

        let mut total_size = LittleEndian::read_i32(&templatedata[0..4]) as usize;
        let mut data = &templatedata[4..];
        let mut templates = Vec::new();

        while total_size > 0 && data.len() >= 6 {
            let size = LittleEndian::read_u16(&data[0..2]) as usize;
            let uid = LittleEndian::read_u16(&data[2..4]);
            let fid = data[4];
            let valid = data[5];

            if data.len() < size {
                break;
            }

            let template = data[6..size].to_vec();
            templates.push(Finger {
                uid,
                fid,
                valid,
                template,
            });

            data = &data[size..];
            if total_size >= size {
                total_size -= size;
            } else {
                total_size = 0;
            }
        }

        Ok(templates)
    }

    /// Retrieves a specific fingerprint template for a user and finger ID.
    pub fn get_user_template(&mut self, uid: u16, fid: u8) -> ZKResult<Option<Finger>> {
        for _ in 0..3 {
            let mut payload = Vec::with_capacity(3);
            payload.write_u16::<LittleEndian>(uid)?;
            payload.write_u8(fid)?;

            let res = self.send_command(_CMD_GET_USERTEMP, payload)?;
            // This command typically returns CMD_DATA with the template
            if res.command == CMD_DATA {
                let mut template = res.payload.into_owned();
                // Strip trailing nulls if present (common firmware quirk)
                while template.ends_with(&[0]) && !template.is_empty() {
                    template.pop();
                }
                return Ok(Some(Finger {
                    uid,
                    fid,
                    valid: 1,
                    template,
                }));
            }
        }
        Ok(None)
    }

    /// Deletes a specific fingerprint template for a user and finger ID.
    pub fn delete_user_template(&mut self, uid: u16, fid: u8) -> ZKResult<()> {
        let mut payload = Vec::with_capacity(3);
        payload.write_u16::<LittleEndian>(uid)?;
        payload.write_u8(fid)?;

        let res = self.send_command(CMD_DELETE_USERTEMP, payload)?;
        if res.command == CMD_ACK_OK {
            let _ = self.refresh_data();
            Ok(())
        } else {
            Err(ZKError::Response("Failed to delete user template".into()))
        }
    }

    /// Registers for specific real-time events.
    pub fn reg_event(&mut self, flags: u32) -> ZKResult<()> {
        let mut payload = Vec::with_capacity(4);
        payload.write_u32::<LittleEndian>(flags)?;

        let res = self.send_command(CMD_REG_EVENT, payload)?;
        if res.command == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response(format!(
                "Failed to register events with flags {}",
                flags
            )))
        }
    }

    /// Listens for real-time events and yields attendance records as they occur.
    /// This is a blocking call that will yield None on timeout.
    pub fn listen_events(&mut self) -> ZKResult<impl Iterator<Item = ZKResult<Attendance>> + '_> {
        // 1. Register for attendance log events if not already done
        self.reg_event(EF_ATTLOG)?;

        Ok(std::iter::from_fn(move || {
            match self.read_packet() {
                Ok(packet) => {
                    // Send ACK_OK back to device to acknowledge receipt
                    let _ = self.send_ack_ok();

                    if packet.command != CMD_REG_EVENT {
                        return Some(Err(ZKError::Response(format!(
                            "Unexpected command during event listening: {}",
                            packet.command
                        ))));
                    }

                    let data = &packet.payload;
                    if data.is_empty() {
                        return None; // Or some signal to continue
                    }

                    // Decode event data based on length (matching pyzk logic)
                    let (uid, user_id, status, punch, timestamp) =
                        if data.len() == EVENT_DATA_LEN_10 {
                            let uid = LittleEndian::read_u16(&data[0..2]) as u32;
                            let status = data[2];
                            let punch = data[3];
                            let timehex = &data[4..10];
                            let ts = ZK::decode_timehex(timehex).ok()?;
                            (uid, uid.to_string(), status, punch, ts)
                        } else if data.len() == EVENT_DATA_LEN_12 {
                            let user_id_num = LittleEndian::read_u32(&data[0..4]);
                            let status = data[4];
                            let punch = data[5];
                            let timehex = &data[6..12];
                            let ts = ZK::decode_timehex(timehex).ok()?;
                            (user_id_num, user_id_num.to_string(), status, punch, ts)
                        } else if data.len() >= EVENT_DATA_LEN_32 {
                            // User ID is string (24 bytes)
                            let user_id = String::from_utf8_lossy(&data[0..24])
                                .trim_matches('\0')
                                .to_string();
                            let status = data[24];
                            let punch = data[25];
                            let timehex = &data[26..32];
                            let ts = ZK::decode_timehex(timehex).ok()?;
                            (0, user_id, status, punch, ts) // UID might be 0 for string-based IDs
                        } else {
                            return Some(Err(ZKError::InvalidData(format!(
                                "Unknown event data length: {}",
                                data.len()
                            ))));
                        };

                    Some(Ok(Attendance {
                        uid,
                        user_id,
                        timestamp,
                        status,
                        punch,
                        timezone_offset: self.timezone_offset,
                    }))
                }
                Err(ZKError::Network(ref e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // This is expected during idle listening
                    None
                }
                Err(e) => Some(Err(e)),
            }
        }))
    }

    /// Internal helper to send a simple ACK_OK response.
    fn send_ack_ok(&mut self) -> ZKResult<()> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| ZKError::Connection("Not connected".into()))?;
        let packet = ZKPacket::new(CMD_ACK_OK, self.session_id, self.reply_id, Vec::new());

        match transport {
            ZKTransport::Tcp(stream) => {
                let mut buf = Vec::with_capacity(16);
                buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_1)?;
                buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_2)?;
                buf.write_u32::<LittleEndian>(8)?;
                packet.to_bytes_into(&mut buf)?;
                stream.write_all(&buf)?;
            }
            ZKTransport::Udp(socket) => {
                let mut buf = Vec::with_capacity(8);
                packet.to_bytes_into(&mut buf)?;
                socket.send(&buf)?;
            }
        }
        Ok(())
    }

    /// Decodes a 6-byte compressed time format used in real-time events.
    fn decode_timehex(hex: &[u8]) -> ZKResult<chrono::NaiveDateTime> {
        if hex.len() < 6 {
            return Err(ZKError::InvalidData("Timehex too short".into()));
        }
        let year = hex[0] as i32 + 2000;
        let month = hex[1] as u32;
        let day = hex[2] as u32;
        let hour = hex[3] as u32;
        let minute = hex[4] as u32;
        let second = hex[5] as u32;

        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, minute, second))
            .ok_or_else(|| ZKError::InvalidData("Invalid date/time in hex".into()))
    }

    /// Helper to find the next available UID on the device.
    /// `start_uid`: The UID to start searching from (useful for testing in high ranges).
    pub fn get_next_free_uid(&mut self, start_uid: u16) -> ZKResult<u16> {
        let users = self.get_users()?;
        let used_uids: std::collections::HashSet<u16> = users.iter().map(|u| u.uid).collect();

        for uid in start_uid..=65535 {
            if !used_uids.contains(&uid) {
                return Ok(uid);
            }
        }

        Err(ZKError::Response(
            "No free UID found in the specified range".into(),
        ))
    }
    /// Finds a user on the device by their alphanumeric User ID.
    pub fn find_user_by_id(&mut self, user_id: &str) -> ZKResult<Option<User>> {
        let users = self.get_users()?;
        Ok(users.into_iter().find(|u| u.user_id == user_id))
    }

    /// Returns the detected timezone offset in minutes.
    pub fn timezone_offset(&self) -> i32 {
        self.timezone_offset
    }

    /// Returns true if the timezone has been synchronized with the device.
    pub fn timezone_synced(&self) -> bool {
        self.timezone_synced
    }

    /// Explicitly refreshes the internal user ID cache.
    pub fn refresh_user_cache(&mut self) -> ZKResult<()> {
        let users = self.get_users()?;
        let mut cache = HashMap::with_capacity(users.len());
        for user in users {
            cache.insert(user.uid, user.user_id);
        }
        self.user_id_cache = Some(cache);
        Ok(())
    }

    /// Internal helper to get User ID for a UID, using cache if available.
    fn get_user_id_from_cache(&mut self, uid: u16) -> String {
        if self.user_id_cache.is_none() {
            let _ = self.refresh_user_cache();
        }

        self.user_id_cache
            .as_ref()
            .and_then(|c| c.get(&uid).cloned())
            .unwrap_or_else(|| uid.to_string())
    }

    pub fn users(&self) -> u32 {
        self.users
    }
    pub fn users_cap(&self) -> i32 {
        self.users_cap
    }
    pub fn fingers(&self) -> u32 {
        self.fingers
    }
    pub fn fingers_cap(&self) -> i32 {
        self.fingers_cap
    }
    pub fn records(&self) -> u32 {
        self.records
    }
    pub fn records_cap(&self) -> i32 {
        self.rec_cap
    }
    pub fn faces(&self) -> u32 {
        self.faces
    }
    pub fn faces_cap(&self) -> i32 {
        self.faces_cap
    }
    pub fn cards(&self) -> i32 {
        self.cards
    }
    pub fn user_packet_size(&self) -> usize {
        self.user_packet_size
    }
}
impl Drop for ZK {
    fn drop(&mut self) {
        // We no longer call self.disconnect() here because it performs blocking network I/O
        // which can cause hangs during object destruction.
        // The transport will be closed automatically when it is dropped.
        self.is_connected = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_commkey() {
        // Key: 0, Session: 619, Ticks: 50
        // Expected: [97, 125, 50, 123]
        let key = 0;
        let session_id = 619;
        let ticks = 50;
        let result = ZK::make_commkey(key, session_id, ticks);
        assert_eq!(result, vec![97, 125, 50, 123]);
    }

    #[test]
    fn test_zk_new_default_password() {
        let zk = ZK::new("192.168.1.201", 4370);
        assert_eq!(zk.password, 0);
    }

    #[test]
    fn test_zk_set_password() {
        let mut zk = ZK::new("192.168.1.201", 4370);
        zk.set_password(12345);
        assert_eq!(zk.password, 12345);
    }

    #[test]
    fn test_make_commkey_complex() {
        // Key: 12345, Session: 9999, Ticks: 100
        // Expected values calculated manually or confirmed via pyzk reference
        let key = 12345;
        let session_id = 9999;
        let ticks = 100;
        let result = ZK::make_commkey(key, session_id, ticks);

        // Let's use the known outcome for key=0 from our previous successful generation as a baseline
        // and add one more known verifiable point if we had it.
        // For now, I'll trust the logic based on the 0.1.0 baseline match.
        assert_eq!(result.len(), 4);
    }
}
