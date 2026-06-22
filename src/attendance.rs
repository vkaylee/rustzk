use std::io::{self, Read, Write};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{ZK, ZKError, ZKResult, ZKErrorCode};
use crate::models::Attendance;
use crate::constants::*;
use crate::transport::ZKTransport;
use crate::protocol::ZKPacket;

impl ZK {
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

        let total_size = LittleEndian::read_u32(&attendance_data[0..4]) as usize;
        if total_size > MAX_RESPONSE_SIZE {
            return Err(ZKError::InvalidData(
                ZKErrorCode::BufferOverflow,
                format!(
                    "Attendance data total_size {} exceeds maximum {}",
                    total_size, MAX_RESPONSE_SIZE
                ),
            ));
        }
        let mut record_size = if self.records > 0 && total_size > 0 {
            total_size / self.records as usize
        } else {
            0
        };

        // Heuristic: Prefer standard record sizes if total_size is a multiple
        if total_size > 0 {
            if total_size.is_multiple_of(40) && (record_size == 0 || record_size.is_multiple_of(40))
            {
                record_size = 40;
            } else if total_size.is_multiple_of(16) && (record_size == 0 || record_size.is_multiple_of(16))
            {
                record_size = 16;
            } else if total_size.is_multiple_of(8) && (record_size == 0 || record_size.is_multiple_of(8))
            {
                record_size = 8;
            }
        }

        let data = &attendance_data[4..];

        let mut attendances = Vec::with_capacity(std::cmp::min(self.records as usize, data.len() / record_size));
        let mut offset = 0;

        let mut uid_cache: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        let mut bytes_cache: std::collections::HashMap<[u8; 24], String> = std::collections::HashMap::new();

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
                let user_id = if let Some(cached) = uid_cache.get(&(uid as u32)) {
                    cached.clone()
                } else {
                    let id = self.get_user_id_from_cache(uid);
                    uid_cache.insert(uid as u32, id.clone());
                    id
                };

                attendances.push(Attendance::new(
                    uid as u32,
                    user_id,
                    timestamp,
                    status,
                    punch,
                    self.timezone_offset,
                ));
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

                let timestamp = ZK::decode_time(&time_bytes)?;
                let uid = user_id_num;
                let user_id = if let Some(cached) = uid_cache.get(&uid) {
                    cached.clone()
                } else {
                    let id = user_id_num.to_string();
                    uid_cache.insert(uid, id.clone());
                    id
                };

                attendances.push(Attendance::new(
                    uid,
                    user_id,
                    timestamp,
                    status,
                    punch,
                    self.timezone_offset,
                ));
                offset += ATT_RECORD_SIZE_16;
            }
        } else if record_size >= ATT_RECORD_SIZE_40 {
            while offset + ATT_RECORD_SIZE_40 <= data.len() {
                let chunk = &data[offset..offset + ATT_RECORD_SIZE_40];
                let mut chunk_ptr = chunk;
                if chunk.starts_with(b"\xff255\x00\x00\x00\x00\x00") {
                    chunk_ptr = &chunk[10..];
                    if chunk_ptr.len() < 30 {
                        break;
                    }
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
                let user_id = if let Some(cached) = bytes_cache.get(&user_id_bytes) {
                    cached.clone()
                } else {
                    let id = String::from_utf8_lossy(&user_id_bytes)
                        .trim_matches('\0')
                        .to_string();
                    bytes_cache.insert(user_id_bytes, id.clone());
                    id
                };

                attendances.push(Attendance::new(
                    uid as u32,
                    user_id,
                    timestamp,
                    status,
                    punch,
                    self.timezone_offset,
                ));
                offset += record_size;
            }
        }

        Ok(attendances)
    }

    /// Registers for specific real-time events.
    pub fn reg_event(&mut self, flags: u32) -> ZKResult<()> {
        let mut payload = [0u8; 4];
        byteorder::LittleEndian::write_u32(&mut payload, flags);

        let res = self.send_command(CMD_REG_EVENT, &payload)?;
        if res.command() == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                format!(
                    "Failed to register events with flags {}",
                    flags
                ),
            ))
        }
    }

    /// Internal helper to send a simple ACK_OK response.
    pub(crate) fn send_ack_ok(&mut self) -> ZKResult<()> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| {
                ZKError::Connection(ZKErrorCode::ConnectionFailed, "Not connected".into())
            })?;
        let packet = ZKPacket::new(CMD_ACK_OK, self.session_id, self.reply_id, &[]);

        match transport {
            ZKTransport::Tcp(ref mut reader) => {
                self.write_buf.clear();
                self.write_buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_1)?;
                self.write_buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_2)?;
                self.write_buf.write_u32::<LittleEndian>(8)?;
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

    /// Decodes a 6-byte compressed time format used in real-time events.
    fn decode_timehex(hex: &[u8]) -> ZKResult<chrono::NaiveDateTime> {
        if hex.len() < 6 {
            return Err(ZKError::InvalidData(
                ZKErrorCode::InvalidDataFormat,
                "Timehex too short".into(),
            ));
        }
        let year = hex[0] as i32 + 2000;
        let month = hex[1] as u32;
        let day = hex[2] as u32;
        let hour = hex[3] as u32;
        let minute = hex[4] as u32;
        let second = hex[5] as u32;

        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, minute, second))
            .ok_or_else(|| {
                ZKError::InvalidData(
                    ZKErrorCode::InvalidDataFormat,
                    "Invalid date/time in hex".into(),
                )
            })
    }

    /// Listens for real-time events and yields attendance records as they occur.
    /// This is a blocking call that will yield None on timeout.
    pub fn listen_events(&mut self) -> ZKResult<impl Iterator<Item = ZKResult<Attendance>> + '_> {
        // 1. Register for attendance log events if not already done
        self.reg_event(EF_ATTLOG)?;

        Ok(std::iter::from_fn(move || {
            loop {
                match self.read_packet() {
                    Ok(packet) => {
                        let _ = self.send_ack_ok();

                        if packet.command() != CMD_REG_EVENT {
                            return Some(Err(ZKError::Response(
                                ZKErrorCode::ProtocolViolation,
                                format!(
                                    "Unexpected command during event listening: {}",
                                    packet.command()
                                ),
                            )));
                        }

                        let data = packet.payload();
                        if data.is_empty() {
                            continue;
                        }

                        // Decode event data based on length (matching pyzk logic)
                        let (uid, user_id, status, punch, timestamp) =
                            if data.len() == EVENT_DATA_LEN_10 {
                                let uid = LittleEndian::read_u16(&data[0..2]) as u32;
                                let status = data[2];
                                let punch = data[3];
                                let timehex = &data[4..10];
                                let ts = match ZK::decode_timehex(timehex) {
                                    Ok(ts) => ts,
                                    Err(e) => return Some(Err(e)),
                                };
                                let user_id = self.get_user_id_from_cache(uid as u16);
                                (uid, user_id, status, punch, ts)
                            } else if data.len() == EVENT_DATA_LEN_12 {
                                let user_id_num = LittleEndian::read_u32(&data[0..4]);
                                let status = data[4];
                                let punch = data[5];
                                let timehex = &data[6..12];
                                let ts = match ZK::decode_timehex(timehex) {
                                    Ok(ts) => ts,
                                    Err(e) => return Some(Err(e)),
                                };
                                let user_id = self.get_user_id_from_cache(user_id_num as u16);
                                (user_id_num, user_id, status, punch, ts)
                            } else if data.len() >= EVENT_DATA_LEN_32 {
                                let user_id = String::from_utf8_lossy(&data[0..24])
                                    .trim_matches('\0')
                                    .to_string();
                                let status = data[24];
                                let punch = data[25];
                                let timehex = &data[26..32];
                                let ts = match ZK::decode_timehex(timehex) {
                                    Ok(ts) => ts,
                                    Err(e) => return Some(Err(e)),
                                };
                                (0, user_id, status, punch, ts)
                            } else {
                                return Some(Err(ZKError::InvalidData(
                                    ZKErrorCode::InvalidDataFormat,
                                    format!("Unknown event data length: {}", data.len()),
                                )));
                            };

                        return Some(Ok(Attendance::new(
                            uid,
                            user_id,
                            timestamp,
                            status,
                            punch,
                            self.timezone_offset,
                        )));
                    }
                    Err(ZKError::Network(ref e))
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        continue;
                    }
                    Err(e) => {
                        return Some(Err(e));
                    }
                }
            }
        }))
    }
}
