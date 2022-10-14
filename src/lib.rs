#![warn(clippy::all)]
#![allow(clippy::pedantic)]
#![forbid(unsafe_code)]

extern crate base64;
extern crate bech32;
extern crate num256;
extern crate num_bigint;
extern crate num_traits;
extern crate ripemd;
extern crate serde;
extern crate sha2;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

pub mod address;
pub mod client;
pub mod coin;
pub mod decimal;
pub mod error;
pub mod mnemonic;
pub mod msg;
pub mod private_key;
pub mod public_key;
pub mod signature;
pub mod utils;

pub use address::Address;
pub use client::Contact;
pub use coin::Coin;
pub use coin::Fee;
pub use mnemonic::Mnemonic;
pub use msg::Msg;
#[cfg(feature = "ethermint")]
pub use private_key::EthermintPrivateKey;
pub use private_key::MessageArgs;
pub use private_key::{CosmosPrivateKey, PrivateKey};
pub use public_key::PublicKey;
pub use signature::Signature;
