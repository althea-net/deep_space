use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::num::ParseIntError;
use std::str;

#[derive(Debug, PartialEq)]
pub enum ByteDecodeError {
    DecodeError(str::Utf8Error),
    ParseError(ParseIntError),
}

impl Display for ByteDecodeError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ByteDecodeError::DecodeError(val) => write!(f, "ByteDecodeError {}", val),
            ByteDecodeError::ParseError(val) => write!(f, "ByteParseError {}", val),
        }
    }
}

impl Error for ByteDecodeError {}

/// A function that takes a hexadecimal representation of bytes
/// back into a stream of bytes.
pub fn hex_str_to_bytes(s: &str) -> Result<Vec<u8>, ByteDecodeError> {
    let s = match s.strip_prefix("0x") {
        Some(v) => v,
        None => s,
    };
    s.as_bytes()
        .chunks(2)
        // .into_iter()
        .map(|ch| {
            str::from_utf8(&ch)
                .map_err(ByteDecodeError::DecodeError)
                .and_then(|res| u8::from_str_radix(&res, 16).map_err(ByteDecodeError::ParseError))
        })
        .collect()
}

pub fn bytes_to_hex_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:0>2x?}", b))
        .fold(String::new(), |acc, x| acc + &x)
}
