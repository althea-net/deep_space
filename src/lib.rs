#![warn(clippy::all)]
#![allow(clippy::pedantic)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate failure;
extern crate bech32;
extern crate num256;
extern crate num_bigint;
extern crate num_traits;
extern crate ripemd160;
extern crate secp256k1;
extern crate serde;
extern crate sha2;
#[macro_use]
extern crate serde_derive;
extern crate base64;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

#[cfg(not(test))]
extern crate serde_json;

pub mod address;
pub mod canonical_json;
pub mod coin;
pub mod msg;
pub mod private_key;
pub mod public_key;
pub mod signature;
pub mod stdfee;
pub mod stdsigndoc;
pub mod stdsignmsg;
pub mod stdtx;
pub mod transaction;
pub mod utils;
