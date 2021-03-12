use crate::utils::hex_str_to_bytes;
use crate::utils::ByteDecodeError;
use bech32::{self, FromBase32, ToBase32};
use serde::Serialize;
use serde::Serializer;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Debug)]
pub enum AddressError {
    Bech32WrongLength,
    Bech32InvalidBase32,
    Bech32InvalidEncoding,
    HexDecodeError(ByteDecodeError),
    HexDecodeErrorWrongLength,
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddressError::Bech32WrongLength => write!(f, "Bech32WrongLength"),
            AddressError::Bech32InvalidBase32 => write!(f, "Bech32InvalidBase32"),
            AddressError::Bech32InvalidEncoding => write!(f, "Bech32InvalidEncoding"),
            AddressError::HexDecodeError(val) => write!(f, "HexDecodeError {}", val),
            AddressError::HexDecodeErrorWrongLength => write!(f, "HexDecodeError Wrong Length"),
        }
    }
}

impl std::error::Error for AddressError {}

impl From<bech32::Error> for AddressError {
    fn from(error: bech32::Error) -> Self {
        match error {
            bech32::Error::InvalidLength => AddressError::Bech32WrongLength,
            bech32::Error::InvalidChar(_) => AddressError::Bech32InvalidBase32,
            bech32::Error::InvalidData(_) => AddressError::Bech32InvalidEncoding,
            bech32::Error::InvalidChecksum => AddressError::Bech32InvalidEncoding,
            bech32::Error::InvalidPadding => AddressError::Bech32InvalidEncoding,
            bech32::Error::MixedCase => AddressError::Bech32InvalidEncoding,
            bech32::Error::MissingSeparator => AddressError::Bech32InvalidEncoding,
        }
    }
}

/// An address that's derived from a given PublicKey
#[derive(Default, PartialEq, Eq, Copy, Clone, Deserialize, Hash)]
pub struct Address([u8; 20]);

impl Address {
    /// Get raw bytes of the address.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(bytes: [u8; 20]) -> Address {
        Address(bytes)
    }

    /// Obtain a bech32 encoded address with a given prefix.
    ///
    /// * `hrp` - A prefix for bech32 encoding. The convention for addresses
    /// in Cosmos is `cosmos`.
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, AddressError> {
        let bech32 = bech32::encode(&hrp.into(), self.0.to_base32())?;
        Ok(bech32)
    }

    /// Parse a bech32 encoded address
    ///
    /// * `s` - A bech32 encoded address
    pub fn from_bech32(s: String) -> Result<Address, AddressError> {
        let (_hrp, data) = match bech32::decode(&s) {
            Ok(val) => val,
            Err(_e) => return Err(AddressError::Bech32InvalidEncoding),
        };
        let vec: Vec<u8> = match FromBase32::from_base32(&data) {
            Ok(val) => val,
            Err(_e) => return Err(AddressError::Bech32InvalidBase32),
        };
        let mut addr = [0u8; 20];
        if vec.len() != 20 {
            return Err(AddressError::Bech32WrongLength);
        }
        addr.copy_from_slice(&vec);
        Ok(Address(addr))
    }
}

impl FromStr for Address {
    type Err = AddressError;
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
                        Err(AddressError::HexDecodeErrorWrongLength)
                    }
                }
                Err(e) => Err(AddressError::HexDecodeError(e)),
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
        let display = self.to_bech32("cosmos").unwrap();
        write!(f, "{}", display).expect("Unable to write");
        Ok(())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_bech32("cosmos").unwrap())
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
