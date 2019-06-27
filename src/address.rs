use bech32::{Bech32, FromBase32, ToBase32};
use failure::Error;
use serde::Serialize;
use serde::Serializer;
use std::fmt::Write;

/// An address that's derived from a given PublicKey
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
pub struct Address([u8; 20]);

impl Address {
    pub fn from_bytes(bytes: [u8; 20]) -> Address {
        Address(bytes)
    }
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        for &byte in self.0.iter() {
            write!(&mut s, "{:02X}", byte).expect("Unable to write");
        }
        s
    }

    /// Obtain a bech32 encoded address with a given prefix.
    ///
    /// * `hrp` - A prefix for bech32 encoding. The convention for addresses
    /// in Cosmos is `cosmos`.
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, Error> {
        let bech32 = Bech32::new(hrp.into(), self.0.to_base32())?;
        Ok(bech32.to_string())
    }

    /// Parse a bech32 encoded address
    ///
    /// * `s` - A bech32 encoded address
    pub fn from_bech32(s: String) -> Result<Address, Error> {
        let bech32: Bech32 = s.parse()?;
        let vec: Vec<u8> = FromBase32::from_base32(bech32.data())?;
        let mut addr = [0u8; 20];
        ensure!(vec.len() == 20, "Wrong size of decoded bech32 data");
        addr.copy_from_slice(&vec);
        Ok(Address(addr))
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
