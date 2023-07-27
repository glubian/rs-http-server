use std::fmt;

use bytes::{Buf, Bytes};

use crate::{Advance as _, transcode::ascii_digit_to_u8};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Malformed;

impl Malformed {
    pub const DESCRIPTION: &str = "version malformed";
}

impl fmt::Display for Malformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Self::DESCRIPTION)
    }
}

impl std::error::Error for Malformed {}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Version(pub u8, pub u8);

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(major, minor) = *self;
        if minor == 0 {
            write!(f, "HTTP/{major}")
        } else {
            write!(f, "HTTP/{major}.{minor}")
        }
    }
}

impl Version {
    pub fn from_bytes(bytes: &mut Bytes) -> Result<Self, Malformed> {
        if bytes.len() < "HTTP/1".len() || !bytes.advance_bytes(b"HTTP/") {
            return Err(Malformed);
        }

        if bytes.len() >= "1.1".len() && bytes.get(1).is_some_and(|&b| b == b'.') {
            let major_digit = ascii_digit_to_u8(bytes[0]);
            let minor_digit = ascii_digit_to_u8(bytes[2]);
            return if major_digit < 10 && minor_digit < 10 {
                bytes.advance(3);
                Ok(Self(major_digit, minor_digit))
            } else {
                Err(Malformed)
            };
        }

        let major_digit = ascii_digit_to_u8(bytes[0]);
        if major_digit < 10 {
            bytes.advance(1);
            Ok(Self(major_digit, 0))
        } else {
            Err(Malformed)
        }
    }

    pub fn write_to_buffer(self, buffer: &mut Vec<u8>) {
        let Self(major, minor) = self;
        let major_char = char::from_digit(major.into(), 10).unwrap() as u8;
        buffer.extend_from_slice(b"HTTP/");
        buffer.push(major_char);
        if minor != 0 {
            let major_char = char::from_digit(minor.into(), 10).unwrap() as u8;
            buffer.push(b'.');
            buffer.push(major_char);
        }
    }

    pub fn to_buffer(self) -> Vec<u8> {
        let capacity = if self.1 == 0 {
            "HTTP/1".len()
        } else {
            "HTTP/1.1".len()
        };
        let mut buffer = Vec::with_capacity(capacity);
        self.write_to_buffer(&mut buffer);
        buffer
    }
}

#[cfg(test)]
mod test {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn from_bytes() {
        let version_1 = Bytes::copy_from_slice(b"HTTP/1");
        let version_1_1 = Bytes::copy_from_slice(b"HTTP/1.1");
        let version_1_ = Bytes::copy_from_slice(b"HTTP/1 ");
        let just_1_1 = Bytes::copy_from_slice(b"1.1");

        assert_eq!(Version::from_bytes(&mut version_1.clone()).unwrap(), Version(1, 0));
        assert_eq!(Version::from_bytes(&mut version_1_1.clone()).unwrap(), Version(1, 1));
        assert_eq!(Version::from_bytes(&mut version_1_.clone()).unwrap(), Version(1, 0));
        assert_eq!(Version::from_bytes(&mut just_1_1.clone()).unwrap_err(), Malformed);
    }

    #[test]
    fn to_buffer() {
        assert_eq!(Version(0, 9).to_buffer().as_slice(), b"HTTP/0.9");
        assert_eq!(Version(1, 0).to_buffer().as_slice(), b"HTTP/1");
        assert_eq!(Version(1, 1).to_buffer().as_slice(), b"HTTP/1.1");
    }
}
