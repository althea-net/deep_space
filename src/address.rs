use crate::utils::hex_str_to_bytes;
use crate::utils::ByteDecodeError;
use bech32::{self, FromBase32, ToBase32};
use failure::Error;
use serde::Serialize;
use serde::Serializer;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Debug)]
pub enum AddressParseError {
    Bech32WrongLength,
    Bech32InvalidBase32,
    Bech32InvalidEncoding,
    HexDecodeError(ByteDecodeError),
    HexDecodeErrorWrongLength,
}

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddressParseError::Bech32WrongLength => write!(f, "Bech32WrongLength"),
            AddressParseError::Bech32InvalidBase32 => write!(f, "Bech32InvalidBase32"),
            AddressParseError::Bech32InvalidEncoding => write!(f, "Bech32InvalidEncoding"),
            AddressParseError::HexDecodeError(val) => write!(f, "HexDecodeError {}", val),
            AddressParseError::HexDecodeErrorWrongLength => {
                write!(f, "HexDecodeError Wrong Length")
            }
        }
    }
}

impl std::error::Error for AddressParseError {}

/// An address that's derived from a given PublicKey
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone, Deserialize)]
pub struct Address([u8; 20]);

impl Address {
    pub fn from_bytes(bytes: [u8; 20]) -> Address {
        Address(bytes)
    }

    /// Obtain a bech32 encoded address with a given prefix.
    ///
    /// * `hrp` - A prefix for bech32 encoding. The convention for addresses
    /// in Cosmos is `cosmos`.
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, Error> {
        let bech32 = bech32::encode(&hrp.into(), self.0.to_base32())?;
        Ok(bech32)
    }

    /// Parse a bech32 encoded address
    ///
    /// * `s` - A bech32 encoded address
    pub fn from_bech32(s: String) -> Result<Address, AddressParseError> {
        let (_hrp, data) = match bech32::decode(&s) {
            Ok(val) => val,
            Err(_e) => return Err(AddressParseError::Bech32InvalidEncoding),
        };
        let vec: Vec<u8> = match FromBase32::from_base32(&data) {
            Ok(val) => val,
            Err(_e) => return Err(AddressParseError::Bech32InvalidBase32),
        };
        let mut addr = [0u8; 20];
        if vec.len() != 20 {
            return Err(AddressParseError::Bech32WrongLength);
        }
        addr.copy_from_slice(&vec);
        Ok(Address(addr))
    }
}

impl FromStr for Address {
    type Err = AddressParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // interpret as bech32 if prefixed, hex otherwise
        if s.starts_with("cosmos1") {
            Address::from_bech32(s.to_string())
        } else {
            match hex_str_to_bytes(s) {
                Ok(bytes) => {
                    if bytes.len() == 20 {
                        let mut inner = [0; 20];
                        inner.copy_from_slice(&bytes[0..20]);
                        Ok(Address(inner))
                    } else {
                        Err(AddressParseError::HexDecodeErrorWrongLength)
                    }
                }
                Err(e) => Err(AddressParseError::HexDecodeError(e)),
            }
        }
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize address as a string with a default prefix for addresses
        let s = self
            .to_bech32("cosmos")
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&s)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for &byte in self.0.iter() {
            write!(f, "{:02X}", byte).expect("Unable to write");
        }
        Ok(())
    }
}

#[test]
fn test_bech32() {
    let address = Address::default();
    assert_eq!(
        address.to_bech32("cosmos").unwrap(),
        "cosmos1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqnrql8a"
    );

    let decoded = Address::from_bech32("cosmos1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqnrql8a".to_string())
        .expect("Unable to decode");
    assert_eq!(address, decoded);
}
