use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

use crate::constants::*;
use crate::protocol::{TCPWrapper, ZKPacket};
use crate::{ZKError, ZKErrorCode, ZKProtocol, ZKResult, ZK};

pub enum ZKTransport {
    Tcp(std::io::BufReader<TcpStream>),
    Udp(UdpSocket),
}

impl ZK {
    pub(crate) fn set_transport_read_timeout(&mut self, timeout: Duration) {
        if let Some(ref mut transport) = self.transport {
            match transport {
                ZKTransport::Tcp(reader) => {
                    let _ = reader.get_ref().set_read_timeout(Some(timeout));
                }
                ZKTransport::Udp(socket) => {
                    let _ = socket.set_read_timeout(Some(timeout));
                }
            }
        }
    }

    pub(crate) fn perform_connect_handshake(&mut self) -> ZKResult<()> {
        self.session_id = 0;
        self.reply_id = USHRT_MAX - 1;
        self.timezone_synced = false;
        self.timezone_offset = 0;

        // Use a short timeout for handshake probes (5s each) so auto-fallback
        // completes in ~10s max instead of waiting the full user-configured timeout.
        let handshake_timeout = Duration::from_secs(5);
        let saved_timeout = self.timeout;
        self.set_transport_read_timeout(handshake_timeout);

        // Attempt 1: Try current checksum algorithm
        let result = self.send_command(CMD_CONNECT, &[]);

        match result {
            Ok(res) => {
                // Restore original timeout for data transfers
                self.set_transport_read_timeout(saved_timeout);
                self.finish_handshake(res)
            }
            Err(ZKError::Network(ref e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // Timeout means the device silently rejected our packet — wrong checksum.
                // Flip to the alternative checksum algorithm and retry.
                log::info!(
                    "Handshake timeout with {} checksum. Retrying with {} checksum...",
                    if self.use_legacy_checksum {
                        "legacy"
                    } else {
                        "default"
                    },
                    if self.use_legacy_checksum {
                        "default"
                    } else {
                        "legacy"
                    }
                );
                self.use_legacy_checksum = !self.use_legacy_checksum;
                self.session_id = 0;
                self.reply_id = USHRT_MAX - 1;

                // Attempt 2: Try alternative checksum algorithm
                let result = self.send_command(CMD_CONNECT, &[]);

                // Restore original timeout for data transfers
                self.set_transport_read_timeout(saved_timeout);

                match result {
                    Ok(res) => {
                        log::info!(
                            "Connected with {} checksum",
                            if self.use_legacy_checksum {
                                "legacy"
                            } else {
                                "default"
                            }
                        );
                        self.finish_handshake(res)
                    }
                    Err(e) => Err(e),
                }
            }
            Err(e) => {
                self.set_transport_read_timeout(saved_timeout);
                Err(e)
            }
        }
    }

    pub(crate) fn finish_handshake(&mut self, res: ZKPacket<'static>) -> ZKResult<()> {
        // Update session_id if we got a valid response (OK or UNAUTH)
        if res.command() == CMD_ACK_OK || res.command() == CMD_ACK_UNAUTH {
            self.session_id = res.session_id();
        }

        if res.command() == CMD_ACK_UNAUTH {
            // Need authentication
            let command_string = ZK::make_commkey(self.password, self.session_id, 50);
            let auth_res = self.send_command(CMD_AUTH, &command_string)?;
            if auth_res.command() == CMD_ACK_UNAUTH {
                self.session_id = 0; // Reset dirty session_id on auth failure
                return Err(ZKError::Connection(
                    ZKErrorCode::Unauthorized,
                    "Unauthorized: Password required or incorrect".into(),
                ));
            }
            self.session_id = auth_res.session_id();
            self.is_connected = true;
            return Ok(());
        }

        if res.command() == CMD_ACK_OK {
            self.is_connected = true;
            Ok(())
        } else {
            Err(ZKError::Connection(
                ZKErrorCode::ProtocolViolation,
                format!(
                    "Invalid response during connect handshake: {}",
                    res.command()
                ),
            ))
        }
    }

    pub(crate) fn sync_timezone(&mut self) -> ZKResult<()> {
        if self.timezone_synced {
            return Ok(());
        }

        // Mark as synced immediately to prevent repeated queries on failure
        self.timezone_synced = true;

        if let Ok(tz_str) = self.get_option_value("TZAdj") {
            if let Ok(tz_val) = tz_str.parse::<i32>() {
                self.timezone_offset = tz_val * 60; // Convert hours to minutes
            }
        }
        Ok(())
    }

    pub(crate) fn read_packet(&mut self) -> ZKResult<ZKPacket<'static>> {
        let transport = self.transport.as_mut().ok_or_else(|| {
            ZKError::Connection(ZKErrorCode::ConnectionFailed, "Not connected".into())
        })?;

        match transport {
            ZKTransport::Tcp(ref mut reader) => {
                let mut read_tcp_frame = || -> ZKResult<ZKPacket<'static>> {
                    let mut header = [0u8; 8];
                    // Read the first chunk (up to 8 bytes). If this blocks or times out,
                    // 0 bytes have been consumed, so we are still in sync.
                    let n = match reader.read(&mut header[..]) {
                        Ok(0) => {
                            return Err(ZKError::Network(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "TCP connection closed (EOF)",
                            )))
                        }
                        Ok(n) => n,
                        Err(e) => return Err(ZKError::Network(e)),
                    };

                    // If we read less than 8 bytes, we must read the rest.
                    // Any error here is a desync because we already consumed `n` bytes.
                    if n < 8 {
                        let mut rest = vec![0u8; 8 - n];
                        if let Err(e) = reader.read_exact(&mut rest) {
                            return Err(ZKError::Network(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("TCP desync: partial header read ({} bytes): {:?}", n, e),
                            )));
                        }
                        header[n..8].copy_from_slice(&rest);
                    }

                    let (length, _) = TCPWrapper::decode_header(&header).map_err(|e| {
                        ZKError::InvalidData(ZKErrorCode::InvalidDataFormat, e.to_string())
                    })?;

                    crate::security::validate_packet_size(length)?;

                    let mut body = vec![0u8; length];
                    // Any error reading the body is a desync since we already read the header.
                    if let Err(e) = reader.read_exact(&mut body) {
                        return Err(ZKError::Network(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "TCP desync: failed to read body of size {}: {:?}",
                                length, e
                            ),
                        )));
                    }

                    let packet = ZKPacket::from_bytes_owned(body)?;
                    if !packet.verify_checksum(self.use_legacy_checksum) {
                        return Err(ZKError::InvalidData(
                            ZKErrorCode::ChecksumMismatch,
                            "Invalid packet checksum".into(),
                        ));
                    }
                    Ok(packet)
                };

                let res = read_tcp_frame();
                if let Err(ref e) = res {
                    let is_timeout = match e {
                        ZKError::Network(io_err) => {
                            io_err.kind() == std::io::ErrorKind::TimedOut
                                || io_err.kind() == std::io::ErrorKind::WouldBlock
                        }
                        _ => false,
                    };
                    if !is_timeout {
                        log::warn!(
                            "TCP stream error (hard failure or desync). Closing transport: {:?}",
                            e
                        );
                        self.is_connected = false;
                        self.transport = None;
                    }
                }
                res
            }
            ZKTransport::Udp(ref mut socket) => {
                self.udp_buf.resize(2048, 0);
                let len = socket.recv(&mut self.udp_buf)?;
                let packet_data = self.udp_buf[..len].to_vec();

                crate::security::validate_packet_size(packet_data.len())?;

                let packet = ZKPacket::from_bytes_owned(packet_data)?;
                if !packet.verify_checksum(self.use_legacy_checksum) {
                    return Err(ZKError::InvalidData(
                        ZKErrorCode::ChecksumMismatch,
                        "Invalid packet checksum".into(),
                    ));
                }
                Ok(packet)
            }
        }
    }

    pub(crate) fn read_response_safe(&mut self) -> ZKResult<ZKPacket<'static>> {
        let mut discarded = 0;
        loop {
            let res_packet = self.read_packet()?;
            log::debug!(
                "Received Packet: Cmd {} (0x{:X}), Reply ID: {}",
                res_packet.command(),
                res_packet.command(),
                res_packet.reply_id()
            );

            if res_packet.reply_id() != self.reply_id {
                discarded += 1;
                log::debug!(
                    "Reply ID mismatch: expected {}, got {}. Discarding packet.",
                    self.reply_id,
                    res_packet.reply_id()
                );
                if discarded > MAX_DISCARDED_PACKETS {
                    return Err(ZKError::Response(
                        ZKErrorCode::ProtocolViolation,
                        "Too many discarded packets".into(),
                    ));
                }
                continue;
            }
            return Ok(res_packet);
        }
    }

    /// Send a ZKPacket over the current transport.
    /// Handles TCP framing (magic + length prefix) and UDP direct send uniformly.
    pub(crate) fn send_packet(&mut self, packet: &ZKPacket<'_>) -> ZKResult<()> {
        let transport = self.transport.as_mut().ok_or_else(|| {
            ZKError::Connection(ZKErrorCode::ConnectionFailed, "Not connected".into())
        })?;

        match transport {
            ZKTransport::Tcp(ref mut reader) => {
                self.write_buf.clear();
                self.write_buf
                    .write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_1)?;
                self.write_buf
                    .write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_2)?;
                self.write_buf
                    .write_u32::<LittleEndian>((packet.payload().len() + 8) as u32)?;
                packet.to_bytes_into(&mut self.write_buf)?;
                reader.get_mut().write_all(&self.write_buf)?;
            }
            ZKTransport::Udp(ref mut socket) => {
                self.write_buf.clear();
                packet.to_bytes_into(&mut self.write_buf)?;
                socket.send(&self.write_buf)?;
            }
        }
        Ok(())
    }

    pub(crate) fn send_command(
        &mut self,
        command: u16,
        payload: &[u8],
    ) -> ZKResult<ZKPacket<'static>> {
        if self.transport.is_none() {
            return Err(ZKError::Connection(
                ZKErrorCode::ConnectionFailed,
                "Not connected".into(),
            ));
        }

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

        let packet = if self.use_legacy_checksum {
            ZKPacket::new_with_legacy(command, self.session_id, self.reply_id, payload)
        } else {
            ZKPacket::new(command, self.session_id, self.reply_id, payload)
        };

        self.send_packet(&packet)?;

        self.read_response_safe()
    }

    fn receive_chunk_into(&mut self, res: ZKPacket<'static>, data: &mut Vec<u8>) -> ZKResult<()> {
        if res.command() == CMD_DATA {
            data.extend_from_slice(res.payload());
            Ok(())
        } else if res.command() == CMD_ACK_OK {
            // New firmware may send ACK_OK before actual data.
            // Give the device a little time to prepare.
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(())
        } else if res.command() == CMD_PREPARE_DATA {
            if res.payload().len() < 4 {
                return Err(ZKError::InvalidData(
                    ZKErrorCode::InvalidDataFormat,
                    "Invalid prepare data payload".into(),
                ));
            }
            let size = byteorder::LittleEndian::read_u32(&res.payload()[..4]) as usize;

            if size > MAX_RESPONSE_SIZE {
                return Err(ZKError::InvalidData(
                    ZKErrorCode::BufferOverflow,
                    format!(
                        "Response size {} exceeds maximum {}",
                        size, MAX_RESPONSE_SIZE
                    ),
                ));
            }

            data.reserve(size);
            let mut remaining = size;

            while remaining > 0 {
                let chunk_res = self.read_response_safe()?;

                if chunk_res.command() == CMD_DATA {
                    data.extend_from_slice(chunk_res.payload());
                    if remaining >= chunk_res.payload().len() {
                        remaining -= chunk_res.payload().len();
                    } else {
                        remaining = 0;
                    }
                } else if chunk_res.command() == CMD_ACK_OK {
                    break;
                } else {
                    return Err(ZKError::Response(
                        ZKErrorCode::ProtocolViolation,
                        format!("Unexpected chunk command: 0x{:X}", chunk_res.command()),
                    ));
                }
            }
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                format!("Invalid response for chunk: 0x{:X}", res.command()),
            ))
        }
    }

    fn read_chunk_into(&mut self, start: i32, size: i32, data: &mut Vec<u8>) -> ZKResult<()> {
        let is_udp = matches!(&self.transport, Some(ZKTransport::Udp(_)));

        let mut payload = [0u8; 8];
        byteorder::LittleEndian::write_i32(&mut payload[0..4], start);
        byteorder::LittleEndian::write_i32(&mut payload[4..8], size);

        let max_attempts = if is_udp { 3 } else { 1 };
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            let res = self.send_command(_CMD_READ_BUFFER, &payload);
            match res {
                Ok(res_packet) => {
                    let receive_res = if res_packet.command() == CMD_ACK_OK {
                        // If we get ACK for read chunk, it means data is coming next.
                        // Wait for the actual data packet.
                        match self.read_response_safe() {
                            Ok(data_packet) => self.receive_chunk_into(data_packet, data),
                            Err(e) => Err(e),
                        }
                    } else {
                        self.receive_chunk_into(res_packet, data)
                    };

                    match receive_res {
                        Ok(()) => return Ok(()),
                        Err(e) => {
                            log::warn!("UDP chunk receive failed on attempt {}: {:?}", attempt, e);
                            last_error = Some(e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("UDP chunk request failed on attempt {}: {:?}", attempt, e);
                    last_error = Some(e);
                }
            }

            if attempt < max_attempts {
                // Short wait before retry (exponential backoff)
                std::thread::sleep(std::time::Duration::from_millis(attempt as u64 * 100));
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ZKError::Response(ZKErrorCode::ProtocolViolation, "Chunk read failed".into())
        }))
    }

    pub(crate) fn read_with_buffer(
        &mut self,
        command: u16,
        fct: u32,
        size: u32,
    ) -> ZKResult<Vec<u8>> {
        let mut payload = [0u8; 11];
        payload[0] = 1;
        LittleEndian::write_u16(&mut payload[1..3], command);
        LittleEndian::write_u32(&mut payload[3..7], fct);
        LittleEndian::write_u32(&mut payload[7..11], size);

        let res = self.send_command(_CMD_PREPARE_BUFFER, &payload)?;
        if res.command() == CMD_DATA {
            return Ok(res.into_payload().into_owned());
        }

        let mut size = if res.payload().len() >= 5 {
            byteorder::LittleEndian::read_u32(&res.payload()[1..5]) as usize
        } else if res.payload().len() >= 4 {
            byteorder::LittleEndian::read_u32(&res.payload()[0..4]) as usize
        } else {
            return Err(ZKError::Response(
                ZKErrorCode::InvalidDataFormat,
                format!("Invalid response size length: {}", res.payload().len()),
            ));
        };

        if res.payload().len() >= 5 && size > MAX_RESPONSE_SIZE {
            let alt_size = byteorder::LittleEndian::read_u32(&res.payload()[0..4]) as usize;
            if alt_size <= MAX_RESPONSE_SIZE {
                size = alt_size;
            }
        }

        if size > MAX_RESPONSE_SIZE {
            return Err(ZKError::InvalidData(
                ZKErrorCode::BufferOverflow,
                format!(
                    "Buffered response size {} exceeds maximum {}",
                    size, MAX_RESPONSE_SIZE
                ),
            ));
        }

        if size == 0 {
            let _ = self.send_command(CMD_FREE_DATA, &[]);
            return Ok(Vec::new());
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
                if empty_responses_count > 20 {
                    return Err(ZKError::Response(
                        ZKErrorCode::Timeout,
                        "Too many empty responses from device during buffer read".into(),
                    ));
                }
                let sleep_ms = std::cmp::min(1 << empty_responses_count, 50);
                std::thread::sleep(std::time::Duration::from_millis(sleep_ms as u64));
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

        let _ = self.send_command(CMD_FREE_DATA, &[]);
        Ok(data)
    }

    pub fn connect(&mut self, protocol: ZKProtocol) -> ZKResult<()> {
        if self.is_connected {
            return Err(ZKError::Connection(
                ZKErrorCode::ConnectionFailed,
                "Already connected. Call disconnect() first.".into(),
            ));
        }
        match protocol {
            ZKProtocol::TCP => self.connect_tcp(),
            ZKProtocol::UDP => self.connect_udp(),
            ZKProtocol::Auto => {
                let saved_checksum = self.use_legacy_checksum;
                match self.connect_tcp() {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::info!("TCP connect failed: {}. Falling back to UDP...", e);
                        self.use_legacy_checksum = saved_checksum;
                        self.connect_udp()
                    }
                }
            }
        }
    }

    fn connect_tcp(&mut self) -> ZKResult<()> {
        let addrs = self.addr.to_socket_addrs().map_err(|e| {
            ZKError::Connection(
                ZKErrorCode::ConnectionFailed,
                format!("Failed to resolve address {}: {}", self.addr, e),
            )
        })?;
        let addr = addrs.into_iter().next().ok_or_else(|| {
            ZKError::Connection(
                ZKErrorCode::ConnectionFailed,
                format!("No address found for {}", self.addr),
            )
        })?;

        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;

        self.transport = Some(ZKTransport::Tcp(std::io::BufReader::new(stream)));
        match self.perform_connect_handshake() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.transport = None;
                Err(e)
            }
        }
    }

    fn connect_udp(&mut self) -> ZKResult<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let socket2_sock = socket2::Socket::from(socket);
        if let Err(e) = socket2_sock.set_recv_buffer_size(2 * 1024 * 1024) {
            log::debug!("Failed to set UDP receive buffer size (SO_RCVBUF): {}", e);
        }
        let socket = std::net::UdpSocket::from(socket2_sock);
        socket.connect(&self.addr)?;
        socket.set_read_timeout(Some(self.timeout))?;
        socket.set_write_timeout(Some(self.timeout))?;

        self.transport = Some(ZKTransport::Udp(socket));
        match self.perform_connect_handshake() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.transport = None;
                Err(e)
            }
        }
    }

    pub(crate) fn send_exit_packet(&mut self) -> ZKResult<()> {
        if self.transport.is_some() {
            self.reply_id = self.reply_id.wrapping_add(1);
            if self.reply_id == USHRT_MAX {
                self.reply_id -= USHRT_MAX;
            }

            let packet = if self.use_legacy_checksum {
                ZKPacket::new_with_legacy(CMD_EXIT, self.session_id, self.reply_id, &[])
            } else {
                ZKPacket::new(CMD_EXIT, self.session_id, self.reply_id, &[])
            };

            self.send_packet(&packet)?;
        }
        Ok(())
    }

    pub fn disconnect(&mut self) -> ZKResult<()> {
        if self.is_connected {
            self.set_transport_read_timeout(Duration::from_secs(3));
            let _ = self.send_command(CMD_EXIT, &[]);
            self.is_connected = false;
        }
        self.transport = None;
        Ok(())
    }
}
