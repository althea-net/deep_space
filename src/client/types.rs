use crate::address::Address;
use cosmos_sdk_proto::cosmos::auth::v1beta1::BaseAccount as ProtoBaseAccount;
use cosmos_sdk_proto::cosmos::vesting::v1beta1::{
    ContinuousVestingAccount, DelayedVestingAccount, PeriodicVestingAccount,
};
use serde::Deserialize;
use tendermint_proto::types::Block;

/// This struct represents the status of a Cosmos chain, instead of just getting the
/// latest block height we mandate that chain status is used, this allows callers to
/// handle the possibility of a halted chain explicitly since essentially all requests
/// about block height come with assumptions about the chains status
pub enum ChainStatus {
    /// The chain is operating correctly and blocks are being produced
    Moving { block_height: u64 },
    /// The chain is operating correctly, but the node we made this request
    /// to is catching up and we should not trust it's responses to be
    /// up to date
    Syncing,
    /// The chain is halted, this node is waiting for the chain to start again
    /// the caller should take appropriate action to await the chain start
    WaitingToStart,
}

/// This struct represents potential responses from the latest block endpoint
/// we can either be syncing, waiting for the chain to start, or have the the
/// actual latest block to the best of the nodes knowledge, which isn't at all
/// a guarantee
pub enum LatestBlock {
    /// The chain is operating correctly and blocks are being produced, this is
    /// the latest one this node has access to
    Latest { block: Block },
    /// The chain is operating correctly, but the node we made this request
    /// to is catching up and we should not trust it's responses to be
    /// up to date
    Syncing { block: Block },
    /// The chain is halted, this node is waiting for the chain to start again
    /// the caller should take appropriate action to await the chain start
    WaitingToStart,
}

/// This is a parsed and validated version of the Cosmos base account proto
/// struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BaseAccount {
    pub address: Address,
    /// an unprocessed proto struct containing a pubkey type
    pub pubkey: Vec<u8>,
    pub account_number: u64,
    pub sequence: u64,
}

impl From<ProtoBaseAccount> for BaseAccount {
    fn from(value: ProtoBaseAccount) -> Self {
        BaseAccount {
            address: value.address.parse().unwrap(),
            pubkey: value.pub_key.unwrap().value,
            account_number: value.account_number,
            sequence: value.sequence,
        }
    }
}

/// A trait for all Cosmos account types that requires
/// all types be sized and implement Clone
pub trait CosmosAccount {
    fn get_base_account(&self) -> BaseAccount;
}

// note that the vesting account nested uses gogoproto's embed tag
// https://github.com/cosmos/cosmos-sdk/blob/master/proto/cosmos/vesting/v1beta1/vesting.proto#L16
// As noted in the gogoproto docs this enforces that the values are not null https://pkg.go.dev/github.com/gogo/protobuf/gogoproto#pkg-types
// therefore we unwrap() the options used to represent Go pointers here

impl CosmosAccount for BaseAccount {
    fn get_base_account(&self) -> BaseAccount {
        self.clone()
    }
}

impl CosmosAccount for ProtoBaseAccount {
    fn get_base_account(&self) -> BaseAccount {
        self.clone().into()
    }
}

impl CosmosAccount for ContinuousVestingAccount {
    fn get_base_account(&self) -> BaseAccount {
        self.base_vesting_account
            .clone()
            .unwrap()
            .base_account
            .unwrap()
            .into()
    }
}

impl CosmosAccount for DelayedVestingAccount {
    fn get_base_account(&self) -> BaseAccount {
        self.base_vesting_account
            .clone()
            .unwrap()
            .base_account
            .unwrap()
            .into()
    }
}

impl CosmosAccount for PeriodicVestingAccount {
    fn get_base_account(&self) -> BaseAccount {
        self.base_vesting_account
            .clone()
            .unwrap()
            .base_account
            .unwrap()
            .into()
    }
}

#[cfg(test)]
mod tests {}
