//! Contains utility functions for interacting with and modifying the Cosmos sdk distribution module
//! including the community pool

use super::{ChainStatus, PAGE};
use crate::client::msgs::{
    MSG_FUND_COMMUNITY_POOL_TYPE_URL, MSG_WITHDRAW_DELEGATOR_REWARD_TYPE_URL,
    MSG_WITHDRAW_VALIDATOR_COMMISSION_TYPE_URL,
};
use crate::error::CosmosGrpcError;
use crate::{Address, Coin, Contact, Msg, PrivateKey};
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmos_sdk_proto::cosmos::base::v1beta1::DecCoin;
use cosmos_sdk_proto::cosmos::distribution::v1beta1::query_client::QueryClient as DistQueryClient;
use cosmos_sdk_proto::cosmos::distribution::v1beta1::{
    MsgFundCommunityPool, QueryValidatorSlashesRequest,
};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::{
    MsgWithdrawDelegatorReward, ValidatorSlashEvent,
};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::{
    MsgWithdrawValidatorCommission, QueryDelegationRewardsRequest,
};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::{
    QueryCommunityPoolRequest, QueryDelegationTotalRewardsRequest,
};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::{
    QueryDelegationTotalRewardsResponse, QueryDelegatorValidatorsRequest,
};
use num256::Uint256;
use num_bigint::ParseBigIntError;
use std::time::Duration;

// required because dec coins are multiplied by 1*10^18
const ONE_ETH: u128 = 10u128.pow(18);

impl Contact {
    /// Gets a list of coins in the community pool, note returned values from this endpoint
    /// are in DecCoins for precision, for the sake of ease of use this endpoint converts them
    /// into their normal form, for easy comparison against any other coin or amount.
    pub async fn query_community_pool(&self) -> Result<Vec<Coin>, CosmosGrpcError> {
        let mut grpc = DistQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let res = grpc.community_pool(QueryCommunityPoolRequest {}).await?;
        let val = res.into_inner().pool;
        let mut res = Vec::new();
        for v in val {
            let parse_result: Result<Uint256, ParseBigIntError> = v.amount.parse();
            match parse_result {
                Ok(parse_result) => res.push(Coin {
                    denom: v.denom,
                    amount: parse_result / ONE_ETH.into(),
                }),
                Err(e) => return Err(CosmosGrpcError::ParseError { error: e }),
            }
        }
        Ok(res)
    }

    /// Gets the slashing events of a validator starting from Genesis to the current block height
    pub async fn query_validator_slashes(
        &self,
        validator_address: impl ToString,
    ) -> Result<Vec<ValidatorSlashEvent>, CosmosGrpcError> {
        let mut grpc = DistQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let current_block = self.get_chain_status().await?;
        let current_block = match current_block {
            ChainStatus::Moving { block_height } => block_height,
            _ => return Err(CosmosGrpcError::ChainNotRunning),
        };

        let res = grpc
            .validator_slashes(QueryValidatorSlashesRequest {
                validator_address: validator_address.to_string(),
                starting_height: 0,
                ending_height: current_block,
                pagination: PAGE,
            })
            .await?
            .into_inner();
        Ok(res.slashes)
    }

    /// Withdraws rewards for the specified delegator to the specified validator
    pub async fn withdraw_delegator_rewards(
        &self,
        validator_address: Address,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let msg = MsgWithdrawDelegatorReward {
            delegator_address: our_address.to_string(),
            validator_address: validator_address.to_string(),
        };

        let msg = Msg::new(MSG_WITHDRAW_DELEGATOR_REWARD_TYPE_URL, msg);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }

    /// gets all the validators a given delegator has delegated to
    pub async fn query_delegator_validators(
        &self,
        delegator_address: Address,
    ) -> Result<Vec<String>, CosmosGrpcError> {
        let mut grpc = DistQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let res = grpc
            .delegator_validators(QueryDelegatorValidatorsRequest {
                delegator_address: delegator_address.to_string(),
            })
            .await?
            .into_inner();
        Ok(res.validators)
    }

    /// gets the rewards for a specific delegation between a single delegator and validator
    pub async fn query_delegation_rewards(
        &self,
        delegator_address: Address,
        validator_address: Address,
    ) -> Result<Vec<DecCoin>, CosmosGrpcError> {
        let mut grpc = DistQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let res = grpc
            .delegation_rewards(QueryDelegationRewardsRequest {
                delegator_address: delegator_address.to_string(),
                validator_address: validator_address.to_string(),
            })
            .await?
            .into_inner()
            .rewards;
        Ok(res)
    }

    /// gets the rewards for a specific delegation between a single delegator and validator
    pub async fn query_all_delegation_rewards(
        &self,
        delegator_address: Address,
    ) -> Result<QueryDelegationTotalRewardsResponse, CosmosGrpcError> {
        let mut grpc = DistQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let res = grpc
            .delegation_total_rewards(QueryDelegationTotalRewardsRequest {
                delegator_address: delegator_address.to_string(),
            })
            .await?
            .into_inner();
        Ok(res)
    }

    /// Withdraws all rewards for the specified delegator across all validators they have
    /// delegated to that are either active or in the process of unbonding
    pub async fn withdraw_all_delegator_rewards(
        &self,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();

        let delegated = self.query_delegator_validators(our_address).await?;

        let mut msgs = Vec::new();

        for val in delegated {
            let msg = MsgWithdrawDelegatorReward {
                delegator_address: our_address.to_string(),
                validator_address: val,
            };
            let msg = Msg::new(MSG_WITHDRAW_DELEGATOR_REWARD_TYPE_URL, msg);
            msgs.push(msg);
        }

        self.send_message(&msgs, None, &[fee], wait_timeout, private_key)
            .await
    }

    /// Withdraws commission from the provided validator
    pub async fn withdraw_validator_commission(
        &self,
        validator_address: Address,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let msg = MsgWithdrawValidatorCommission {
            validator_address: validator_address.to_string(),
        };

        let msg = Msg::new(MSG_WITHDRAW_VALIDATOR_COMMISSION_TYPE_URL, msg);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }

    /// Sends the specified funds directly to the community pool
    pub async fn fund_community_pool(
        &self,
        amount: Vec<Coin>,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let msg = MsgFundCommunityPool {
            amount: amount.into_iter().map(|a| a.into()).collect(),
            depositor: our_address.to_string(),
        };

        let msg = Msg::new(MSG_FUND_COMMUNITY_POOL_TYPE_URL, msg);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }
}
