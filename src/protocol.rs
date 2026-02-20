use crate::constants::*;
use crate::{ZKError, ZKResult};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::borrow::Cow;
use std::io::{self, Cursor};

/// Calculates the ZK protocol checksum for the given data.
pub fn calculate_checksum(data: &[u8]) -> u16 {
    let mut checksum: u32 = 0;
    let mut i = 0;
    let len = data.len();

    while i + 1 < len {
        let val = u16::from_le_bytes([data[i], data[i + 1]]);
        checksum += val as u32;
        if checksum > USHRT_MAX as u32 {
            checksum -= USHRT_MAX as u32;
        }
        i += 2;
    }

    if i < len {
        checksum += data[i] as u32;
        if checksum > USHRT_MAX as u32 {
            checksum -= USHRT_MAX as u32;
        }
    }

    while checksum > USHRT_MAX as u32 {
        checksum -= USHRT_MAX as u32;
    }

    let mut checksum = !(checksum as i32);
    while checksum < 0 {
        checksum += USHRT_MAX as i32;
    }

    checksum as u16
}

/// Represents a ZK protocol packet.
#[derive(Debug, Clone)]
pub struct ZKPacket<'a> {
    /// The command code (e.g., CMD_CONNECT).
    pub command: u16,
    /// The packet checksum.
    pub checksum: u16,
    /// The session ID allocated by the device.
    pub session_id: u16,
    /// The reply ID for tracking request-response pairs.
    pub reply_id: u16,
    /// The raw payload of the command.
    pub payload: Cow<'a, [u8]>,
}

impl<'a> ZKPacket<'a> {
    /// Creates a new ZKPacket and automatically calculates the checksum.
    pub fn new(
        command: u16,
        session_id: u16,
        reply_id: u16,
        payload: impl Into<Cow<'a, [u8]>>,
    ) -> Self {
        let mut packet = ZKPacket {
            command,
            checksum: 0,
            session_id,
            reply_id,
            payload: payload.into(),
        };
        packet.checksum = packet.calculate_checksum();
        packet
    }

    fn calculate_checksum(&self) -> u16 {
        let mut sum: u32 = 0;

        // Sum the header fields (excluding the checksum field itself)
        sum += self.command as u32;
        sum += self.session_id as u32;
        sum += self.reply_id as u32;

        // Sum the payload bytes as u16 pairs
        let mut i = 0;
        let payload_len = self.payload.len();
        while i + 1 < payload_len {
            let val = u16::from_le_bytes([self.payload[i], self.payload[i + 1]]);
            sum += val as u32;
            if sum > USHRT_MAX as u32 {
                sum -= USHRT_MAX as u32;
            }
            i += 2;
        }

        if i < payload_len {
            sum += self.payload[i] as u32;
            if sum > USHRT_MAX as u32 {
                sum -= USHRT_MAX as u32;
            }
        }

        while sum > USHRT_MAX as u32 {
            sum -= USHRT_MAX as u32;
        }

        let mut checksum = !(sum as i32);
        while checksum < 0 {
            checksum += USHRT_MAX as i32;
        }

        checksum as u16
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + self.payload.len());
        let _ = self.to_bytes_into(&mut buf);
        buf
    }

    pub fn to_bytes_into(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        buf.write_u16::<LittleEndian>(self.command)?;
        buf.write_u16::<LittleEndian>(self.checksum)?;
        buf.write_u16::<LittleEndian>(self.session_id)?;
        buf.write_u16::<LittleEndian>(self.reply_id)?;
        buf.extend_from_slice(&self.payload);
        Ok(())
    }

    pub fn from_bytes(data: &'a [u8]) -> io::Result<Self> {
        if data.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Packet too short",
            ));
        }
        let mut rdr = Cursor::new(data);
        let command = rdr.read_u16::<LittleEndian>()?;
        let checksum = rdr.read_u16::<LittleEndian>()?;
        let session_id = rdr.read_u16::<LittleEndian>()?;
        let reply_id = rdr.read_u16::<LittleEndian>()?;
        let payload = Cow::Borrowed(&data[8..]);
        Ok(ZKPacket {
            command,
            checksum,
            session_id,
            reply_id,
            payload,
        })
    }

    pub fn from_bytes_owned(mut data: Vec<u8>) -> ZKResult<Self> {
        if data.len() < 8 {
            return Err(ZKError::InvalidData("Packet too short".into()));
        }
        let mut rdr = Cursor::new(&data);
        let command = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
        let checksum = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
        let session_id = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
        let reply_id = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
        data.drain(0..8);
        Ok(ZKPacket {
            command,
            checksum,
            session_id,
            reply_id,
            payload: Cow::Owned(data),
        })
    }
}

pub struct TCPWrapper;

impl TCPWrapper {
    pub fn wrap(packet: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + packet.len());
        let _ = Self::wrap_into(packet, &mut buf);
        buf
    }

    pub fn wrap_into(packet: &[u8], buf: &mut Vec<u8>) -> io::Result<()> {
        buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_1)?;
        buf.write_u16::<LittleEndian>(MACHINE_PREPARE_DATA_2)?;
        buf.write_u32::<LittleEndian>(packet.len() as u32)?;
        buf.extend_from_slice(packet);
        Ok(())
    }

    pub fn unwrap(data: &[u8]) -> io::Result<(&[u8], usize)> {
        let (_length, total_len) = Self::decode_header(data)?;

        if data.len() < total_len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "TCP packet incomplete",
            ));
        }

        Ok((&data[8..total_len], total_len))
    }

    pub fn decode_header(data: &[u8]) -> io::Result<(usize, usize)> {
        if data.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "TCP header too short",
            ));
        }
        let mut rdr = Cursor::new(data);
        let magic1 = rdr.read_u16::<LittleEndian>()?;
        let magic2 = rdr.read_u16::<LittleEndian>()?;
        let length = rdr.read_u32::<LittleEndian>()? as usize;

        if magic1 != MACHINE_PREPARE_DATA_1 || magic2 != MACHINE_PREPARE_DATA_2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid TCP magic numbers",
            ));
        }

        Ok((length, 8 + length))
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;

    #[test]
    fn test_calculate_checksum() {
        // Sample packet: Connect command (1000), session 0, reply USHRT_MAX-1
        // Header: [E8, 03, 00, 00, 00, 00, FE, FF]
        // CMD: 1000 (0x03E8)
        // Checksum placeholder: 0
        // Session: 0
        // Reply: 65534 (0xFFFE)
        let data = vec![0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0xFE, 0xFF];
        let checksum = calculate_checksum(&data);
        // Sum: 0x03E8 + 0x0000 + 0x0000 + 0xFFFE = 0x103E6
        // Adjusted: 0x103E6 - 0xFFFF = 0x03E7 (999)
        // Python style NOT: 65534 - 999 = 64535 (0xFC17)
        assert_eq!(checksum, 0xFC17);
    }

    #[test]
    fn test_zk_packet_new() {
        let packet = ZKPacket::new(1000, 0, 65534, vec![]);
        assert_eq!(packet.command, 1000);
        assert_eq!(packet.checksum, 0xFC17);
        assert_eq!(packet.session_id, 0);
        assert_eq!(packet.reply_id, 65534);
    }

    #[test]
    fn test_zk_packet_serialization() {
        let packet = ZKPacket::new(1000, 0, 65534, vec![0x01, 0x02]);
        let bytes = packet.to_bytes();
        let decoded = ZKPacket::from_bytes(&bytes).unwrap();
        assert_eq!(packet.command, decoded.command);
        assert_eq!(packet.checksum, decoded.checksum);
        assert_eq!(packet.payload, decoded.payload);
    }

    #[test]
    fn test_tcp_wrapper() {
        let packet = vec![0xAA, 0xBB];
        let wrapped = TCPWrapper::wrap(&packet);
        assert_eq!(wrapped.len(), 10); // 8 bytes header + 2 bytes data
        let (unwrapped, total_len) = TCPWrapper::unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, packet);
        assert_eq!(total_len, 10);
    }
}
