pub mod constants;
pub mod models;
pub mod protocol;

use std::io::{self, Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::time::Duration;
use thiserror::Error;
use byteorder::{ReadBytesExt, WriteBytesExt, ByteOrder};

use crate::constants::*;
use crate::protocol::{ZKPacket, TCPWrapper};
use crate::models::{User, Attendance};

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

pub struct ZK {
    addr: String,
    transport: Option<ZKTransport>,
    session_id: u16,
    reply_id: u16,
    timeout: Duration,
    pub is_connected: bool,
    pub user_packet_size: usize,
    pub users: i32,
    pub fingers: i32,
    pub records: i32,
    pub cards: i32,
    pub faces: i32,
    pub fingers_cap: i32,
    pub users_cap: i32,
    pub rec_cap: i32,
    pub faces_cap: i32,
    pub encoding: String,
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
        }
    }

    pub fn connect(&mut self, tcp: bool) -> ZKResult<()> {
        if tcp {
            let stream = TcpStream::connect(&self.addr)?;
            stream.set_read_timeout(Some(self.timeout))?;
            stream.set_write_timeout(Some(self.timeout))?;
            self.transport = Some(ZKTransport::Tcp(stream));
        } else {
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            socket.connect(&self.addr)?;
            socket.set_read_timeout(Some(self.timeout))?;
            socket.set_write_timeout(Some(self.timeout))?;
            self.transport = Some(ZKTransport::Udp(socket));
        }

        self.session_id = 0;
        self.reply_id = USHRT_MAX - 1;

        let res = self.send_command(CMD_CONNECT, Vec::new())?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_UNAUTH {
            self.session_id = res.session_id;
            if res.command == CMD_ACK_UNAUTH {
                return Err(ZKError::Connection("Unauthorized: Password required or incorrect".into()));
            }
            self.is_connected = true;
            Ok(())
        } else {
            Err(ZKError::Connection(format!("Invalid response: {}", res.command)))
        }
    }

    pub fn send_command(&mut self, command: u16, payload: Vec<u8>) -> ZKResult<ZKPacket> {
        let packet = ZKPacket::new(command, self.session_id, self.reply_id, payload);
        let bytes = packet.to_bytes();

        let transport = self.transport.as_mut().ok_or_else(|| ZKError::Connection("Not connected".into()))?;

        match transport {
            ZKTransport::Tcp(stream) => {
                let wrapped = TCPWrapper::wrap(&bytes);
                stream.write_all(&wrapped)?;

                let mut header = [0u8; 8];
                stream.read_exact(&mut header)?;
                let (length, _total_len) = TCPWrapper::decode_header(&header).map_err(|e| ZKError::InvalidData(e.to_string()))?;
                
                let mut body = vec![0u8; length];
                stream.read_exact(&mut body)?;

                let res_packet = ZKPacket::from_bytes(&body)?;
                self.reply_id = res_packet.reply_id;
                Ok(res_packet)
            }
            ZKTransport::Udp(socket) => {
                socket.send(&bytes)?;
                let mut buf = vec![0u8; 2048];
                let len = socket.recv(&mut buf)?;
                let res_packet = ZKPacket::from_bytes(&buf[..len])?;
                self.reply_id = res_packet.reply_id;
                Ok(res_packet)
            }
        }
    }

    pub fn read_sizes(&mut self) -> ZKResult<()> {
        let res = self.send_command(CMD_GET_FREE_SIZES, Vec::new())?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            let data = res.payload;
            if data.len() >= 80 {
                let mut rdr = io::Cursor::new(&data[..80]);
                let mut fields = [0i32; 20];
                for i in 0..20 {
                    fields[i] = rdr.read_i32::<byteorder::LittleEndian>()?;
                }
                self.users = fields[4];
                self.fingers = fields[6];
                self.records = fields[8];
                self.cards = fields[12];
                self.fingers_cap = fields[14];
                self.users_cap = fields[15];
                self.rec_cap = fields[16];
            }
            if data.len() >= 92 {
                let mut rdr = io::Cursor::new(&data[80..92]);
                self.faces = rdr.read_i32::<byteorder::LittleEndian>()?;
                let _ = rdr.read_i32::<byteorder::LittleEndian>()?;
                self.faces_cap = rdr.read_i32::<byteorder::LittleEndian>()?;
            }
            Ok(())
        } else {
            Err(ZKError::Response(format!("Failed to read sizes: {}", res.command)))
        }
    }

    pub fn decode_time(t: &[u8]) -> ZKResult<chrono::NaiveDateTime> {
        if t.len() < 4 {
            return Err(ZKError::InvalidData("Timestamp too short".into()));
        }
        let mut rdr = io::Cursor::new(t);
        let t = rdr.read_u32::<byteorder::LittleEndian>()?;
        
        let second = (t % 60) as u32;
        let t = t / 60;
        let minute = (t % 60) as u32;
        let t = t / 60;
        let hour = (t % 24) as u32;
        let t = t / 24;
        let day = (t % 31 + 1) as u32;
        let t = t / 31;
        let month = (t % 12 + 1) as u32;
        let t = t / 12;
        let year = (t + 2000) as i32;

        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d: chrono::NaiveDate| d.and_hms_opt(hour, minute, second))
            .ok_or_else(|| ZKError::InvalidData("Invalid date/time".into()))
    }

    fn receive_chunk(&mut self, res: ZKPacket) -> ZKResult<Vec<u8>> {
        if res.command == CMD_DATA {
            Ok(res.payload)
        } else if res.command == CMD_PREPARE_DATA {
            if res.payload.len() < 4 {
                return Err(ZKError::InvalidData("Invalid prepare data payload".into()));
            }
            let size = byteorder::LittleEndian::read_u32(&res.payload[..4]) as usize;
            
            if size > MAX_RESPONSE_SIZE {
                return Err(ZKError::InvalidData(format!("Response size {} exceeds maximum {}", size, MAX_RESPONSE_SIZE)));
            }

            let mut data = Vec::with_capacity(size);
            let mut remaining = size;

            while remaining > 0 {
                let transport = self.transport.as_mut().ok_or_else(|| ZKError::Connection("Not connected".into()))?;
                let chunk_res = match transport {
                    ZKTransport::Tcp(stream) => {
                        let mut header = [0u8; 8];
                        stream.read_exact(&mut header)?;
                        let (length, _total_len) = TCPWrapper::decode_header(&header).map_err(|e| ZKError::InvalidData(e.to_string()))?;

                        let mut body = vec![0u8; length];
                        stream.read_exact(&mut body)?;
                        ZKPacket::from_bytes(&body)?
                    }
                    ZKTransport::Udp(socket) => {
                        let mut buf = vec![0u8; 2048];
                        let len = socket.recv(&mut buf)?;
                        ZKPacket::from_bytes(&buf[..len])?
                    }
                };

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
                    return Err(ZKError::Response(format!("Unexpected chunk command: {}", chunk_res.command)));
                }
            }
            Ok(data)
        } else {
            Err(ZKError::Response(format!("Invalid response for chunk: {}", res.command)))
        }
    }

    fn read_chunk(&mut self, start: i32, size: i32) -> ZKResult<Vec<u8>> {
        let mut payload = Vec::new();
        payload.write_i32::<byteorder::LittleEndian>(start)?;
        payload.write_i32::<byteorder::LittleEndian>(size)?;

        let res = self.send_command(_CMD_READ_BUFFER, payload)?;
        self.receive_chunk(res)
    }

    pub fn read_with_buffer(&mut self, command: u16, fct: u8, ext: u32) -> ZKResult<Vec<u8>> {
        let mut payload = Vec::new();
        payload.write_u8(1)?; // ZK6/8 flag?
        payload.write_u16::<byteorder::LittleEndian>(command)?;
        payload.write_u8(fct)?;
        payload.write_u32::<byteorder::LittleEndian>(ext)?;

        let res = self.send_command(_CMD_PREPARE_BUFFER, payload)?;
        if res.command == CMD_DATA {
            return Ok(res.payload);
        }

        if res.command != CMD_PREPARE_DATA {
             // Some devices might return ACK_OK and wait for read_chunk
             // But usually it's PREPARE_DATA
        }

        let size = if res.payload.len() >= 5 {
            byteorder::LittleEndian::read_u32(&res.payload[1..5]) as usize
        } else {
            0
        };

        if size > MAX_RESPONSE_SIZE {
            return Err(ZKError::InvalidData(format!("Buffered response size {} exceeds maximum {}", size, MAX_RESPONSE_SIZE)));
        }

        let max_chunk = if let Some(ZKTransport::Tcp(_)) = self.transport {
            TCP_MAX_CHUNK
        } else {
            UDP_MAX_CHUNK
        };

        let mut data = Vec::with_capacity(size);
        let mut start = 0;
        let mut remaining = size;

        while remaining > 0 {
            let chunk_size = std::cmp::min(remaining, max_chunk);
            let chunk = self.read_chunk(start as i32, chunk_size as i32)?;
            data.extend_from_slice(&chunk);
            start += chunk.len();
            if remaining >= chunk.len() {
                remaining -= chunk.len();
            } else {
                remaining = 0;
            }
        }

        // Free data buffer
        let _ = self.send_command(CMD_FREE_DATA, Vec::new());

        Ok(data)
    }

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

        if self.user_packet_size == 28 {
            while offset + 28 <= data.len() {
                let chunk = &data[offset..offset + 28];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let privilege = rdr.read_u8()?;
                let mut password_bytes = [0u8; 5];
                rdr.read_exact(&mut password_bytes)?;
                let mut name_bytes = [0u8; 8];
                rdr.read_exact(&mut name_bytes)?;
                let card = rdr.read_u32::<byteorder::LittleEndian>()?;
                let group_id = rdr.read_u8()?;
                let _timezone = rdr.read_u16::<byteorder::LittleEndian>()?;
                let user_id = rdr.read_u32::<byteorder::LittleEndian>()?;

                users.push(User {
                    uid,
                    name: String::from_utf8_lossy(&name_bytes).trim_matches('\0').to_string(),
                    privilege,
                    password: String::from_utf8_lossy(&password_bytes).trim_matches('\0').to_string(),
                    group_id: group_id.to_string(),
                    user_id: user_id.to_string(),
                    card,
                });
                offset += 28;
            }
        } else if self.user_packet_size == 72 {
             while offset + 72 <= data.len() {
                let chunk = &data[offset..offset + 72];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let privilege = rdr.read_u8()?;
                let mut password_bytes = [0u8; 8];
                rdr.read_exact(&mut password_bytes)?;
                let mut name_bytes = [0u8; 24];
                rdr.read_exact(&mut name_bytes)?;
                let card = rdr.read_u32::<byteorder::LittleEndian>()?;
                let mut group_id_bytes = [0u8; 7]; // Wait, let me double check the unpack
                rdr.read_exact(&mut group_id_bytes)?;
                let mut user_id_bytes = [0u8; 24];
                rdr.read_exact(&mut user_id_bytes)?;

                users.push(User {
                    uid,
                    name: String::from_utf8_lossy(&name_bytes).trim_matches('\0').to_string(),
                    privilege,
                    password: String::from_utf8_lossy(&password_bytes).trim_matches('\0').to_string(),
                    group_id: String::from_utf8_lossy(&group_id_bytes).trim_matches('\0').to_string(),
                    user_id: String::from_utf8_lossy(&user_id_bytes).trim_matches('\0').to_string(),
                    card,
                });
                offset += 72;
            }
        }

        Ok(users)
    }

    pub fn get_attendance(&mut self) -> ZKResult<Vec<Attendance>> {
        self.read_sizes()?;
        if self.records == 0 {
            return Ok(Vec::new());
        }

        let users = self.get_users()?;
        let attendance_data = self.read_with_buffer(CMD_ATTLOG_RRQ, 0, 0)?;
        if attendance_data.len() < 4 {
            return Ok(Vec::new());
        }

        let total_size = byteorder::LittleEndian::read_u32(&attendance_data[0..4]) as usize;
        let record_size = total_size / self.records as usize;
        let data = &attendance_data[4..];

        let mut attendances = Vec::new();
        let mut offset = 0;

        if record_size == 8 {
            while offset + 8 <= data.len() {
                let chunk = &data[offset..offset + 8];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let status = rdr.read_u8()?;
                let mut time_bytes = [0u8; 4];
                rdr.read_exact(&mut time_bytes)?;
                let punch = rdr.read_u8()?;

                let timestamp = ZK::decode_time(&time_bytes)?;
                let user_id = users.iter()
                    .find(|u| u.uid == uid)
                    .map(|u| u.user_id.clone())
                    .unwrap_or_else(|| uid.to_string());

                attendances.push(Attendance {
                    uid: uid as u32,
                    user_id,
                    timestamp,
                    status,
                    punch,
                });
                offset += 8;
            }
        } else if record_size == 16 {
            while offset + 16 <= data.len() {
                let chunk = &data[offset..offset + 16];
                let mut rdr = io::Cursor::new(chunk);
                let user_id_num = rdr.read_u32::<byteorder::LittleEndian>()?;
                let mut time_bytes = [0u8; 4];
                rdr.read_exact(&mut time_bytes)?;
                let status = rdr.read_u8()?;
                let punch = rdr.read_u8()?;
                // reserved 2 bytes, workcode 4 bytes
                
                let timestamp = ZK::decode_time(&time_bytes)?;
                let user_id = user_id_num.to_string();
                let uid = users.iter()
                    .find(|u| u.user_id == user_id)
                    .map(|u| u.uid as u32)
                    .unwrap_or(user_id_num);

                attendances.push(Attendance {
                    uid,
                    user_id,
                    timestamp,
                    status,
                    punch,
                });
                offset += 16;
            }
        } else if record_size >= 40 {
            while offset + 40 <= data.len() {
                let chunk = &data[offset..offset + 40];
                // Handle the 0xff255 prefix if present as in Python code
                let mut chunk_ptr = chunk;
                if chunk.starts_with(b"\xff255\x00\x00\x00\x00\x00") {
                    chunk_ptr = &chunk[10..];
                    if chunk_ptr.len() < 30 { break; } // Should not happen if record_size >= 40
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
                let user_id = String::from_utf8_lossy(&user_id_bytes).trim_matches('\0').to_string();

                attendances.push(Attendance {
                    uid: uid as u32,
                    user_id,
                    timestamp,
                    status,
                    punch,
                });
                offset += record_size;
            }
        }

        Ok(attendances)
    }

    pub fn get_firmware_version(&mut self) -> ZKResult<String> {
        let res = self.send_command(CMD_GET_VERSION, Vec::new())?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            Ok(String::from_utf8_lossy(&res.payload).trim_matches('\0').to_string())
        } else {
            Err(ZKError::Response("Can't read firmware version".into()))
        }
    }

    fn get_option_value(&mut self, key: &str) -> ZKResult<String> {
        let mut command_string = key.as_bytes().to_vec();
        command_string.push(0);
        let res = self.send_command(CMD_OPTIONS_RRQ, command_string)?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            let data = String::from_utf8_lossy(&res.payload).trim_matches('\0').to_string();
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

    pub fn get_time(&mut self) -> ZKResult<chrono::NaiveDateTime> {
        let res = self.send_command(CMD_GET_TIME, Vec::new())?;
        if res.command == CMD_ACK_OK || res.command == CMD_ACK_DATA {
            ZK::decode_time(&res.payload[..4])
        } else {
            Err(ZKError::Response("Can't get time".into()))
        }
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
            self.send_command(CMD_EXIT, Vec::new())?;
            self.is_connected = false;
        }
        self.transport = None;
        Ok(())
    }
}
