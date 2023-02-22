use crate::error::AddressError;
use crate::utils::contains_non_hex_chars;
use crate::utils::hex_str_to_bytes;
use crate::utils::ArrayString;
use bech32::{self, FromBase32};
use bech32::{ToBase32, Variant};
use core::fmt::Display;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

#[cfg(feature = "ethermint")]
use clarity::address::Address as EthAddress;

/// In cases where it's impossible to know the Bech32 prefix
/// we fall back to this value
pub const DEFAULT_PREFIX: &str = "cosmos";

/// An address representing a Cosmos Account, which is chain specific.
/// These are typically encoded using Bech32, where the `prefix` field forms the Bech32 `hrp`,
/// while for Protobuf transport they are encoded as Base64 byte strings.
///
/// Addresses have variable length depending on their purpose, the Base variant is the most common
#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub enum Address {
    /// A regular account derived from a PrivateKey, or a plain Module account. Has a 20 byte buffer.
    Base(BaseAddress),
    /// An account derived from a Module account and a key. Has a 32 byte buffer.
    Derived(DerivedAddress),
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Address, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decoded = Address::from_bech32(s);
        match decoded {
            Ok(d) => Ok(d),
            Err(e) => Err(serde::de::Error::custom(e.to_string())),
        }
    }
}

/// An address that's derived from a given PublicKey, has the typical 20 bytes of data
#[derive(PartialEq, Eq, Copy, Clone, Hash, Deserialize, Serialize)]
pub struct BaseAddress {
    bytes: [u8; 20],
    prefix: ArrayString,
}

/// An address that's derived from a module account, has a larger 32 byte buffer since it is the
/// result of a 32 byte SHA256 hash
///
/// Notably, this is needed for interchain accounts, which are derived from the ICA module account,
/// but liquidity pools and incentives are very likely to use these as well.
/// Example: https://github.com/cosmos/ibc-go/blob/v3.3.0/modules/apps/27-interchain-accounts/types/account.go#L42-L47
#[derive(PartialEq, Eq, Copy, Clone, Hash, Deserialize, Serialize)]
pub struct DerivedAddress {
    bytes: [u8; 32],
    prefix: ArrayString,
}

impl Address {
    /// Read a slice and a prefix into an account Address
    pub fn from_slice<T: Into<String>>(bytes: &[u8], prefix: T) -> Result<Self, AddressError> {
        match bytes.len() {
            20 => {
                let mut result = [0u8; 20];
                result.copy_from_slice(bytes);
                Ok(Address::Base(BaseAddress::from_bytes(result, prefix)?))
            }
            32 => {
                let mut result = [0u8; 32];
                result.copy_from_slice(bytes);
                Ok(Address::Derived(DerivedAddress::from_bytes(
                    result, prefix,
                )?))
            }
            _ => Err(AddressError::BytesDecodeErrorWrongLength),
        }
    }

    /// Parse a bech32 encoded address
    ///
    /// * `s` - A bech32 encoded address
    pub fn from_bech32(s: String) -> Result<Self, AddressError> {
        let (hrp, data, _) = match bech32::decode(&s) {
            Ok(val) => val,
            Err(_) => {
                return Err(AddressError::Bech32InvalidEncoding);
            }
        };
        let vec: Vec<u8> = match FromBase32::from_base32(&data) {
            Ok(val) => val,
            Err(_e) => return Err(AddressError::Bech32InvalidBase32),
        };
        match vec.len() {
            20 => {
                let mut addr = [0u8; 20];
                addr.copy_from_slice(&vec);
                Ok(Address::Base(BaseAddress::from_bytes(addr, &hrp)?))
            }
            32 => {
                let mut addr = [0u8; 32];
                addr.copy_from_slice(&vec);
                Ok(Address::Derived(DerivedAddress::from_bytes(addr, &hrp)?))
            }
            _ => Err(AddressError::Bech32WrongLength),
        }
    }

    /// Encodes `bytes` and `prefix` into a Bech32 String
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, AddressError> {
        let bech32 = bech32::encode(&hrp.into(), self.get_bytes().to_base32(), Variant::Bech32)?;
        Ok(bech32)
    }

    /// Changes the `prefix` field to modify the resulting Bech32 `hrp`
    pub fn change_prefix<T: Into<String>>(&mut self, prefix: T) -> Result<(), AddressError> {
        match self {
            Address::Base(base_address) => {
                base_address.prefix = ArrayString::new(&prefix.into())?;
            }
            Address::Derived(derived_address) => {
                derived_address.prefix = ArrayString::new(&prefix.into())?;
            }
        }
        Ok(())
    }

    /// Returns the underlying `bytes` buffer as a slice
    pub fn get_bytes(&self) -> &[u8] {
        match self {
            Address::Base(base_address) => &base_address.bytes,
            Address::Derived(derived_address) => &derived_address.bytes,
        }
    }

    /// Returns the underlying `bytes` buffer as a Vec
    pub fn to_vec(&self) -> Vec<u8> {
        self.get_bytes().to_vec()
    }

    /// Returns the current `prefix`, used as the Bech32 `hrp`
    pub fn get_prefix(&self) -> String {
        match self {
            Address::Base(base_address) => base_address.prefix,
            Address::Derived(derived_address) => derived_address.prefix,
        }
        .to_string()
    }
}

impl FromStr for Address {
    type Err = AddressError;

    /// Parse an address from a string as bech32 OR as a base64 hex string
    fn from_str(s: &str) -> Result<Self, AddressError> {
        // interpret as bech32 we find any non-hex chars, hex otherwise
        if contains_non_hex_chars(s) {
            Address::from_bech32(s.to_string())
        } else {
            match hex_str_to_bytes(s) {
                Ok(bytes) => Address::from_slice(&bytes, DEFAULT_PREFIX),
                Err(e) => Err(AddressError::HexDecodeError(e)),
            }
        }
    }
}
impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let display = self.to_bech32(self.get_prefix()).unwrap();
        write!(f, "{display}").expect("Unable to write");
        Ok(())
    }
}
impl Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_bech32(self.get_prefix()).unwrap())
    }
}

impl BaseAddress {
    pub fn from_bytes<T: Into<String>>(bytes: [u8; 20], prefix: T) -> Result<Self, AddressError> {
        Ok(Self {
            bytes,
            prefix: ArrayString::new(&prefix.into())?,
        })
    }
}

impl DerivedAddress {
    pub fn from_bytes<T: Into<String>>(bytes: [u8; 32], prefix: T) -> Result<Self, AddressError> {
        Ok(Self {
            bytes,
            prefix: ArrayString::new(&prefix.into())?,
        })
    }
}

// Locally computes the address for a Cosmos ModuleAccount, which is the first 20 bytes of
// the sha256 hash of the name of the module.
// See Module() for more info: https://github.com/cosmos/cosmos-sdk/blob/main/types/address/hash.go
//
// Note: some accounts like the Distribution module's "fee_collector" work the same way,
// despite the fact that "fee_collector" is not a module
pub fn get_module_account_address(
    module_name: &str,
    prefix: Option<&str>,
) -> Result<Address, AddressError> {
    let prefix = prefix.unwrap_or(DEFAULT_PREFIX);

    // create a Sha256 object
    let mut hasher = Sha256::new();
    hasher.update(module_name.as_bytes());
    let result = hasher.finalize();

    Address::from_slice(&result[0..20], prefix)
}

#[cfg(feature = "ethermint")]
// Swaps the byte interpretation of an address from CosmosAddress to EthAddress
pub fn cosmos_address_to_eth_address(
    address: Address,
) -> Result<EthAddress, clarity::error::Error> {
    EthAddress::from_slice(address.get_bytes())
}

#[test]
fn test_bech32() {
    let address = Address::from_slice(&[0; 20], "cosmos").unwrap();
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
    Address::from_slice(&[0; 20], DEFAULT_PREFIX).unwrap();
}

#[test]
fn test_parse() {
    let address = Address::from_slice(&[0; 20], "cosmos").unwrap();

    let decoded = "cosmos1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqnrql8a"
        .parse()
        .expect("Unable to decode");
    assert_eq!(address, decoded);

    let _test: Address = "cosmos1vlms2r8f6x7yxjh3ynyzc7ckarqd8a96ckjvrp"
        .parse()
        .unwrap();
}
