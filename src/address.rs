use bech32::{Bech32, ToBase32};
use failure::Error;
use std::fmt::Write;

#[derive(Default)]
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
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, Error> {
        let bech32 = Bech32::new(hrp.into(), self.0.to_base32())?;
        Ok(bech32.to_string())
    }
}

#[test]
fn test_bech32() {
    let address = Address::default();
    assert_eq!(
        address.to_bech32("cosmos").unwrap(),
        "cosmos1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqnrql8a"
    );
}
