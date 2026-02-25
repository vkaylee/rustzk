use crate::constants::*;
use crate::{ZKError, ZKResult};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::borrow::Cow;
use std::io::{self, Cursor};

/// Calculates the ZK protocol checksum for raw byte data (default algorithm).
///
/// This uses the Python pyzk-aligned algorithm: `!(sum as i32)` then adding
/// `USHRT_MAX` until non-negative. This produces a result that is exactly 1 less
/// than the legacy Rust bitwise NOT approach.
///
/// For packet-level checksums, prefer [`ZKPacket::new`] or [`ZKPacket::new_with_legacy`].
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
    /// Creates a new ZKPacket and automatically calculates the checksum
    /// using the **default** (Python pyzk-aligned) algorithm.
    ///
    /// # Checksum Algorithm
    ///
    /// The default algorithm uses signed negation: `!(sum as i32)` then adds
    /// `USHRT_MAX` (65535) until non-negative. For a sum of 999, this produces:
    /// `!(999 as i32) = -1000`, then `-1000 + 65535 = 64535 (0xFC17)`.
    ///
    /// Compatible with firmware that follows the Python pyzk reference implementation.
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

    /// Creates a new ZKPacket using the **legacy** checksum algorithm.
    ///
    /// # Checksum Algorithm
    ///
    /// The legacy algorithm uses Rust's unsigned bitwise NOT: `!(sum as u16)`.
    /// For a sum of 999, this produces: `!999u16 = 65535 - 999 = 64536 (0xFC18)`.
    ///
    /// This is exactly **1 greater** than the default algorithm for all inputs.
    ///
    /// # Firmware Compatibility
    ///
    /// Required by older ZKTeco firmware that validates checksums strictly:
    /// - **ZAM180_TFT** (Ver 6.60 Aug 19 2021) — confirmed via real device test
    /// - Likely other firmware released before ~2020
    ///
    /// These devices silently drop packets with the default checksum (no error
    /// response), causing connection timeouts.
    pub fn new_with_legacy(
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
        packet.checksum = packet.calculate_checksum_legacy();
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

        // Final step: signed negation (Python pyzk-compatible).
        // Python: ~sum produces -(sum+1), then adding 65535 gives (65534 - sum).
        // This differs by exactly 1 from the legacy u16 NOT approach (65535 - sum).
        let mut checksum = !(sum as i32);
        while checksum < 0 {
            checksum += USHRT_MAX as i32;
        }

        checksum as u16
    }

    /// Legacy checksum algorithm (v0.4.4 and earlier).
    ///
    /// Uses Rust's native unsigned bitwise NOT on u16: `!(sum as u16) = 65535 - sum`.
    /// This produces a checksum exactly **1 greater** than the default algorithm.
    ///
    /// # Why two algorithms?
    ///
    /// Python's `~x` on a signed int gives `-(x+1)`, which after adding 65535
    /// yields `65534 - x`. Rust's `!x` on a u16 yields `65535 - x`. The 1-bit
    /// difference causes some firmware (e.g., ZAM180) to reject packets that
    /// use the default algorithm.
    fn calculate_checksum_legacy(&self) -> u16 {
        let mut sum: u32 = 0;

        sum += self.command as u32;
        sum += self.session_id as u32;
        sum += self.reply_id as u32;

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

        !(sum as u16)
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

        let (command, checksum, session_id, reply_id) = {
            let mut rdr = Cursor::new(&data);
            let cmd = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
            let chk = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
            let sid = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
            let rid = rdr.read_u16::<LittleEndian>().map_err(ZKError::from)?;
            (cmd, chk, sid, rid)
        };

        // split_off is more efficient as it doesn't shift the entire vector
        let payload = data.split_off(8);

        Ok(ZKPacket {
            command,
            checksum,
            session_id,
            reply_id,
            payload: Cow::Owned(payload),
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
