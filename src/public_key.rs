use crate::address::Address;
use failure::Error;
use ripemd160::{Digest as Ripemd160Digest, Ripemd160};
use sha2::{Digest, Sha256};

pub struct PublicKey([u8; 33]);

impl PublicKey {
    pub fn from_bytes(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_address(&self) -> Result<Address, Error> {
        let sha256 = Sha256::digest(&self.0);
        let ripemd160 = Ripemd160::digest(&sha256);
        let mut bytes: [u8; 20] = Default::default();
        bytes.copy_from_slice(&ripemd160[..]);
        Ok(Address::from_bytes(bytes))
    }
}
