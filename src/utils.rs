use crate::error::{ArrayStringError, ByteDecodeError};
use crate::Coin;
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
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

/// An enum
#[derive(PartialEq, Eq, Clone, Hash, Deserialize, Serialize, Debug)]
pub enum FeeInfo {
    InsufficientFees { min_fees: Vec<Coin> },
    InsufficientGas { amount: u64 },
}

/// Returns what fee related problem is keeping your tx from running, you may need
/// to run this more than once because the simulator only returns one error at a time.
/// returns None if there are no fee related errors
/// This is more brittle than it needs to be because the simulate endpoint (A) returns only one
/// problem at a time and (B) returns insufficient fee messages as a memo, not an error type
pub fn determine_min_fees_and_gas(input: &TxResponse) -> Option<FeeInfo> {
    if input.raw_log.contains("insufficient_fees") || input.raw_log.contains("insufficient fee") {
        let parts = input.raw_log.split(':').nth(2);
        if let Some(amounts) = parts {
            let mut coins = Vec::new();
            for item in amounts.split(',') {
                if let Ok(coin) = item.parse() {
                    coins.push(coin);
                }
            }
            Some(FeeInfo::InsufficientFees { min_fees: coins })
        } else {
            error!("Failed parsing insufficient fee error, probably changed gRPC error message response");
            None
        }
    } else if input.gas_used > input.gas_wanted {
        Some(FeeInfo::InsufficientGas {
            amount: input.gas_used as u64,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_determine_fees() {
        let below_min_fees_tx_response = TxResponse {
            height: 0,
            txhash: "3B07E4A68F2260717E45F4469CC197DBC2637858C33B4790B83F4AE9FC058570".to_string(),
            codespace: "sdk".to_string(),
            code: 13,
            data: String::new(),
            raw_log: "insufficient fees; got: 1gravity0xD50c0953a99325d01cca655E57070F1be4983b6b required: 50000ualtg,250000ufootoken: insufficient fee".to_string(),
            logs: Vec::new(),
            info: String::new(),
            gas_used: 0,
            gas_wanted: 0,
            tx: None,
            timestamp: String::new(),
        };
        let correct_output = Some(FeeInfo::InsufficientFees {
            min_fees: vec![
                Coin {
                    denom: "ualtg".to_string(),
                    amount: 50000u64.into(),
                },
                Coin {
                    denom: "ufootoken".to_string(),
                    amount: 250000u64.into(),
                },
            ],
        });
        assert_eq!(
            determine_min_fees_and_gas(&below_min_fees_tx_response),
            correct_output
        );
    }
}
