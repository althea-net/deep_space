use crate::address::Address;
use crate::error::CosmosGrpcError;
use bytes::BytesMut;
use cosmos_sdk_proto::cosmos::auth::v1beta1::{BaseAccount as ProtoBaseAccount, ModuleAccount};
use cosmos_sdk_proto::cosmos::vesting::v1beta1::{
    ContinuousVestingAccount, DelayedVestingAccount, PeriodicVestingAccount, PermanentLockedAccount,
};
use cosmos_sdk_proto::tendermint::types::Block;
use prost::Message;
use prost_types::Any;

/// This struct represents the status of a Cosmos chain, instead of just getting the
/// latest block height we mandate that chain status is used, this allows callers to
/// handle the possibility of a halted chain explicitly since essentially all requests
/// about block height come with assumptions about the chains status
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// Wrapper representing the various account types and their metadata data, easier to use than traits
/// when you want to pass data around
#[derive(Debug, Clone)]
pub enum AccountType {
    ProtoBaseAccount(ProtoBaseAccount),
    PeriodicVestingAccount(PeriodicVestingAccount),
    ContinuousVestingAccount(ContinuousVestingAccount),
    DelayedVestingAccount(DelayedVestingAccount),
    ModuleAccount(ModuleAccount),
    PermenantLockedAccount(PermanentLockedAccount),
}

impl AccountType {
    pub fn get_base_account(&self) -> BaseAccount {
        match self {
            AccountType::ProtoBaseAccount(a) => a.get_base_account(),
            AccountType::PeriodicVestingAccount(a) => a.get_base_account(),
            AccountType::ContinuousVestingAccount(a) => a.get_base_account(),
            AccountType::DelayedVestingAccount(a) => a.get_base_account(),
            AccountType::ModuleAccount(a) => a.get_base_account(),
            AccountType::PermenantLockedAccount(a) => a.get_base_account(),
        }
    }

    pub fn decode_from_any(value: prost_types::Any) -> Result<Self, CosmosGrpcError> {
        let mut buf = BytesMut::with_capacity(value.value.len());
        buf.extend_from_slice(&value.value);
        match (
            ProtoBaseAccount::decode(buf.clone()),
            ContinuousVestingAccount::decode(buf.clone()),
            PeriodicVestingAccount::decode(buf.clone()),
            DelayedVestingAccount::decode(buf.clone()),
            ModuleAccount::decode(buf.clone()),
            PermanentLockedAccount::decode(buf.clone()),
        ) {
            (Ok(d), _, _, _, _, _) => Ok(AccountType::ProtoBaseAccount(d)),
            // delayed and continuous can be parsed incorrectly
            (_, Ok(c), Ok(p), _, _, _) => {
                if value.type_url.contains("Continuous") {
                    Ok(AccountType::ContinuousVestingAccount(c))
                } else {
                    Ok(AccountType::PeriodicVestingAccount(p))
                }
            }
            (_, Ok(d), _, _, _, _) => Ok(AccountType::ContinuousVestingAccount(d)),
            (_, _, Ok(d), _, _, _) => Ok(AccountType::PeriodicVestingAccount(d)),
            (_, _, _, Ok(d), _, _) => Ok(AccountType::DelayedVestingAccount(d)),
            (_, _, _, _, Ok(d), _) => Ok(AccountType::ModuleAccount(d)),
            (_, _, _, _, _, Ok(d)) => Ok(AccountType::PermenantLockedAccount(d)),
            (Err(e), _, _, _, _, _) => Err(CosmosGrpcError::DecodeError { error: e }),
        }
    }
}

/// This is a parsed and validated version of the Cosmos base account proto
/// struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BaseAccount {
    pub address: Address,
    /// an unprocessed proto struct containing a pubkey type
    #[serde(skip_serializing, skip_deserializing)]
    pub pubkey: Option<Any>,
    pub account_number: u64,
    pub sequence: u64,
}

impl From<ProtoBaseAccount> for BaseAccount {
    fn from(value: ProtoBaseAccount) -> Self {
        BaseAccount {
            address: value.address.parse().unwrap(),
            pubkey: value.pub_key,
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

impl CosmosAccount for ModuleAccount {
    fn get_base_account(&self) -> BaseAccount {
        self.base_account.clone().unwrap().into()
    }
}

impl CosmosAccount for PermanentLockedAccount {
    fn get_base_account(&self) -> BaseAccount {
        self.base_vesting_account
            .clone()
            .unwrap()
            .base_account
            .unwrap()
            .into()
    }
}

/// A mirror of the BlockParams struct represents the maximum gas and bytes a block is allowed in the chain
/// None represents unlimited
#[derive(Debug, Clone)]
pub struct BlockParams {
    pub max_bytes: u64,
    pub max_gas: Option<u64>,
}

#[cfg(test)]
mod tests {}
