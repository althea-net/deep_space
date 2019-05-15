use crate::address::Address;
use failure::Error;

pub struct PublicKey;

impl PublicKey {
    pub fn address(&self) -> Result<Address, Error> {
        unimplemented!("address");
    }
}
