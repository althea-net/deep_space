use crate::address::Address;
use failure::Error;

pub struct PublicKey([u8; 33]);

impl PublicKey {
    pub fn from_bytes(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl PublicKey {
    pub fn address(&self) -> Result<Address, Error> {
        unimplemented!("address");
    }
}
