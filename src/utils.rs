use crate::error::{ArrayStringError, ByteDecodeError};
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::{str, usize};

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

#[derive(PartialEq, Eq, Copy, Clone, Hash, Deserialize, Serialize)]
pub struct ArrayString {
    chars: [Option<char>; ArrayString::MAX_LEN],
    used: usize,
}

impl ArrayString {
    const MAX_LEN: usize = 32;

    pub fn new(input: &str) -> Result<Self, ArrayStringError> {
        if input.len() > ArrayString::MAX_LEN {
            Err(ArrayStringError::TooLong)
        } else {
            let mut ret: [Option<char>; ArrayString::MAX_LEN] = [None; ArrayString::MAX_LEN];
            let mut counter = 0;
            for char in input.chars() {
                ret[counter] = Some(char);
                counter += 1;
            }
            Ok(ArrayString {
                chars: ret,
                used: counter,
            })
        }
    }
}

impl Display for ArrayString {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut str = String::new();
        for c in self.chars.iter() {
            if let Some(v) = c {
                str.push(*v)
            } else {
                break;
            }
        }
        write!(f, "{}", str)
    }
}

pub fn contains_non_hex_chars(input: &str) -> bool {
    for char in input.chars() {
        if !char.is_ascii_hexdigit() {
            return true;
        }
    }
    false
}
