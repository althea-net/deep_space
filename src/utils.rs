use crate::error::{ArrayStringError, ByteDecodeError, CosmosGrpcError, SdkErrorCode};
use crate::Coin;
use bytes::BytesMut;
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use prost::{DecodeError, Message};
use prost_types::Any;
use sha2::Digest;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::str;
use std::time::Duration;
use tonic::metadata::AsciiMetadataValue;
use tonic::{IntoRequest, Request};

/// Converts a standard GRPC query Request struct into a historical one at the given `past_height` by adding
/// the "x-cosmos-block-height" gRPC metadata to the request
/// `req` should be a standard GRPC request like cosmos_sdk_proto_althea::cosmos::bank::v1beta1::QueryBalancesRequest
///
/// Returns a Request with the set gRPC metadata
pub fn historical_grpc_query<T>(req: impl IntoRequest<T>, past_height: u64) -> Request<T> {
    let mut request = req.into_request();
    request.metadata_mut().insert(
        "x-cosmos-block-height",
        AsciiMetadataValue::from(past_height),
    );
    request
}

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
            str::from_utf8(ch)
                .map_err(ByteDecodeError::DecodeError)
                .and_then(|res| u8::from_str_radix(res, 16).map_err(ByteDecodeError::ParseError))
        })
        .collect()
}

pub fn bytes_to_hex_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:0>2x?}"))
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
        write!(f, "{str}")
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

/// CosmosSDK txhashes are the sha256 hash of the signed protobuf tx bytes encoded as uppercase hex
/// Returns the txhash as a String
pub fn get_txhash(input: Vec<u8>) -> String {
    let hash = sha2::Sha256::digest(&input);
    bytes_to_hex_str(&hash).to_uppercase()
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
    // obvious gas problem
    if input.gas_used > input.gas_wanted {
        return Some(FeeInfo::InsufficientGas {
            amount: input.gas_used as u64,
        });
    }
    // now we interpret the error and see if we can't figure out more
    // is this an sdk error? If it's not we won't have a gas error
    if input.codespace == "sdk" {
        if let Some(err) = SdkErrorCode::from_code(input.code) {
            if err == SdkErrorCode::ErrInsufficientFee {
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
            } else {
                // some error other than fees
                None
            }
        } else {
            // no error, nothing to do!
            None
        }
    } else {
        // some non-sdk error
        None
    }
}

/// Checks a tx response code for known issues returns true if tx is good, false if the tx
/// has some known error
pub fn check_for_sdk_error(input: &TxResponse) -> Result<(), CosmosGrpcError> {
    // check for gas errors, in this case no txid is retured because the tx never made it to the mempool
    if let Some(v) = determine_min_fees_and_gas(input) {
        return Err(CosmosGrpcError::InsufficientFees { fee_info: v });
    }

    // check for known errors in the sdk codespace, if the error is module
    // specific we will not detect it and the error will go un-noticed
    if input.codespace == "sdk" {
        if let Some(e) = SdkErrorCode::from_code(input.code) {
            return Err(CosmosGrpcError::TransactionFailed {
                tx: input.clone(),
                time: Duration::from_secs(0),
                sdk_error: Some(e),
                tonic_code: None,
            });
        }
    }

    Ok(())
}

/// Helper function for encoding the the proto any type
pub fn encode_any(input: impl prost::Message, type_url: impl Into<String>) -> Any {
    let mut value = Vec::new();
    input.encode(&mut value).unwrap();
    Any {
        type_url: type_url.into(),
        value,
    }
}

pub fn decode_any<T: Message + Default>(any: Any) -> Result<T, DecodeError> {
    let bytes = any.value;

    decode_bytes(bytes)
}

pub fn decode_bytes<T: Message + Default>(bytes: Vec<u8>) -> Result<T, DecodeError> {
    let mut buf = BytesMut::with_capacity(bytes.len());
    buf.extend_from_slice(&bytes);

    // Here we use the `T` type to decode whatever type of message this attestation holds
    // for use in the `f` function
    T::decode(buf)
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
            events: Vec::new(),
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
