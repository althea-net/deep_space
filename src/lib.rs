#![warn(clippy::all)]
#![allow(clippy::pedantic)]
#![forbid(unsafe_code)]

extern crate base64;
extern crate bech32;
extern crate num256;
extern crate num_bigint;
extern crate num_traits;
extern crate ripemd160;
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
pub use private_key::MessageArgs;
pub use private_key::PrivateKey;
pub use public_key::PublicKey;
pub use signature::Signature;

use cosmos_sdk_proto::cosmos::auth::v1beta1::BaseAccount;
use cosmos_sdk_proto::cosmos::vesting::v1beta1::{PeriodicVestingAccount,DelayedVestingAccount,ContinuousVestingAccount};


pub enum CosmosAccount{
    BaseAccount(BaseAccount),
    PeriodicVesting(PeriodicVestingAccount),
    DelayedVesting(DelayedVestingAccount),
    ContinuousVesting(ContinuousVestingAccount),
}

impl CosmosAccount{
    pub fn get_sequence(&self) -> Option<u64>{
        match self{
            CosmosAccount::BaseAccount(ba) => Some(ba.sequence),
            CosmosAccount::ContinuousVesting(cv)=>{
                match &cv.base_vesting_account{
                    Some(ba)=>{
                        match &ba.base_account{
                            Some(ba) => Some(ba.sequence),
                            None => None,
                        }
                    }
                    None => None,
                }
            }
            CosmosAccount::PeriodicVesting(pv) => {
                match &pv.base_vesting_account{
                    Some(ba)=>{
                        match &ba.base_account{
                            Some(ba) => Some(ba.sequence),
                            None => None,
                        }
                    }
                    None => None,
                }
            },
            CosmosAccount::DelayedVesting(dv) => {
                match &dv.base_vesting_account{
                    Some(ba)=>{
                        match &ba.base_account{
                            Some(ba) => Some(ba.sequence),
                            None => None,
                        }
                    }
                    None => None,
                }
            },
        }
    }

    pub fn get_account_number(&self) -> Option<u64>{
        match self{
            CosmosAccount::BaseAccount(ba) => Some(ba.account_number),
            CosmosAccount::ContinuousVesting(cv)=>{
                match &cv.base_vesting_account{
                    Some(ba)=>{
                        match &ba.base_account{
                            Some(ba) => Some(ba.account_number),
                            None => None,
                        }
                    }
                    None => None,
                }
            }
            CosmosAccount::PeriodicVesting(pv) => {
                match &pv.base_vesting_account{
                    Some(ba)=>{
                        match &ba.base_account{
                            Some(ba) => Some(ba.account_number),
                            None => None,
                        }
                    }
                    None => None,
                }
            },
            CosmosAccount::DelayedVesting(dv) => {
                match &dv.base_vesting_account{
                    Some(ba)=>{
                        match &ba.base_account{
                            Some(ba) => Some(ba.account_number),
                            None => None,
                        }
                    }
                    None => None,
                }
            },
        }
    }
}