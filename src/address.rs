use crate::error::AddressError;
use crate::utils::contains_non_hex_chars;
use crate::utils::hex_str_to_bytes;
use crate::utils::ArrayString;
use bech32::{self, FromBase32};
use bech32::{ToBase32, Variant};
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

/// An address that's derived from a given PublicKey
#[derive(PartialEq, Eq, Copy, Clone, Hash, Deserialize, Serialize)]
pub struct Address {
    bytes: [u8; 20],
    prefix: ArrayString,
}

impl Address {
    /// In cases where it's impossible to know the Bech32 prefix
    /// we fall back to this value
    pub const DEFAULT_PREFIX: &'static str = "cosmos";

    pub fn from_slice<T: Into<String>>(bytes: &[u8], prefix: T) -> Result<Address, AddressError> {
        if bytes.len() != 20 {
            return Err(AddressError::BytesDecodeErrorWrongLength);
        }
        let mut result = [0u8; 20];
        result.copy_from_slice(bytes);
        Address::from_bytes(result, prefix)
    }

    pub fn from_bytes<T: Into<String>>(
        bytes: [u8; 20],
        prefix: T,
    ) -> Result<Address, AddressError> {
        Ok(Address {
            bytes,
            prefix: ArrayString::new(&prefix.into())?,
        })
    }

    /// Returns bytes of a given Address  as a slice of bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }

    pub fn get_prefix(&self) -> String {
        self.prefix.to_string()
    }

    pub fn change_prefix<T: Into<String>>(&mut self, prefix: T) -> Result<(), AddressError> {
        self.prefix = ArrayString::new(&prefix.into())?;
        Ok(())
    }

    /// Obtain a bech32 encoded address with a given prefix.
    ///
    /// * `hrp` - A prefix for bech32 encoding. The convention for addresses
    /// in Cosmos is `cosmos`.
    /// note this does not update the prefix stored in the address
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, AddressError> {
        let bech32 = bech32::encode(&hrp.into(), self.bytes.to_base32(), Variant::Bech32)?;
        Ok(bech32)
    }

    /// Parse a bech32 encoded address
    ///
    /// * `s` - A bech32 encoded address
    pub fn from_bech32(s: String) -> Result<Address, AddressError> {
        let (hrp, data, _) = match bech32::decode(&s) {
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
        Address::from_bytes(addr, &hrp)
    }
}

impl FromStr for Address {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // interpret as bech32 we find any non-hex chars, hex otherwise
        if contains_non_hex_chars(s) {
            Address::from_bech32(s.to_string())
        } else {
            match hex_str_to_bytes(s) {
                Ok(bytes) => {
                    if bytes.len() == 20 {
                        let mut inner = [0; 20];
                        inner.copy_from_slice(&bytes[0..20]);
                        Ok(Address::from_bytes(inner, Address::DEFAULT_PREFIX)?)
                    } else {
                        Err(AddressError::HexDecodeErrorWrongLength)
                    }
                }
                Err(e) => Err(AddressError::HexDecodeError(e)),
            }
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let display = self.to_bech32(self.get_prefix()).unwrap();
        write!(f, "{}", display).expect("Unable to write");
        Ok(())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_bech32(self.get_prefix()).unwrap())
    }
}

#[test]
fn test_bech32() {
    let address = Address::from_bytes([0; 20], "cosmos").unwrap();
    assert_eq!(
        address.to_bech32("cosmos").unwrap(),
        "cosmos1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqnrql8a"
    );

    let decoded = Address::from_bech32("cosmos1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqnrql8a".to_string())
        .expect("Unable to decode");
    assert_eq!(address, decoded);

    Address::from_bech32("cosmos1vlms2r8f6x7yxjh3ynyzc7ckarqd8a96ckjvrp".to_string())
        .expect("Failed to decode");
}

#[test]
fn test_default_prefix() {
    Address::from_bytes([0; 20], Address::DEFAULT_PREFIX).unwrap();
}

#[test]
fn test_parse() {
    let address = Address::from_bytes([0; 20], "cosmos").unwrap();

    let decoded = "cosmos1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqnrql8a"
        .parse()
        .expect("Unable to decode");
    assert_eq!(address, decoded);

    let _test: Address = "cosmos1vlms2r8f6x7yxjh3ynyzc7ckarqd8a96ckjvrp"
        .parse()
        .unwrap();
}
