//! Contains utility functions for interacting with and submitting Cosmos governance proposals

use super::PAGE;
use crate::client::msgs::{
    MSG_BEGIN_REDELEGATE_TYPE_URL, MSG_DELEGATE_TYPE_URL, MSG_UNDELEGATE_TYPE_URL,
};
use crate::error::CosmosGrpcError;
use crate::Address;
use crate::Coin;
use crate::Contact;
use crate::Msg;
use crate::PrivateKey;
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmos_sdk_proto::cosmos::staking::v1beta1::query_client::QueryClient as StakingQueryClient;
use cosmos_sdk_proto::cosmos::staking::v1beta1::DelegationResponse;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgBeginRedelegate;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgUndelegate;
use cosmos_sdk_proto::cosmos::staking::v1beta1::QueryDelegationRequest;
use cosmos_sdk_proto::cosmos::staking::v1beta1::QueryValidatorDelegationsRequest;
use cosmos_sdk_proto::cosmos::staking::v1beta1::QueryValidatorsRequest;
use cosmos_sdk_proto::cosmos::staking::v1beta1::Validator;
use std::time::Duration;

impl Contact {
    /// Gets a list of validators
    pub async fn get_validators_list(
        &self,
        filters: QueryValidatorsRequest,
    ) -> Result<Vec<Validator>, CosmosGrpcError> {
        let mut grpc = StakingQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();

        let res = grpc.validators(filters).await?.into_inner().validators;
        Ok(res)
    }

    /// Gets a list of bonded validators
    pub async fn get_active_validators(&self) -> Result<Vec<Validator>, CosmosGrpcError> {
        let req = QueryValidatorsRequest {
            pagination: PAGE,
            status: "BOND_STATUS_BONDED".to_string(),
        };
        self.get_validators_list(req).await
    }

    /// Gets a list of delegators who have delegated to this validator
    pub async fn get_validator_delegations(
        &self,
        validator: Address,
    ) -> Result<Vec<DelegationResponse>, CosmosGrpcError> {
        let mut grpc = StakingQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();

        let res = grpc
            .validator_delegations(QueryValidatorDelegationsRequest {
                validator_addr: validator.to_string(),
                pagination: PAGE,
            })
            .await?
            .into_inner()
            .delegation_responses;
        Ok(res)
    }

    /// Gets a the delegation info for a given delegator and validator pair
    pub async fn get_delegation(
        &self,
        validator: Address,
        delegator: Address,
    ) -> Result<Option<DelegationResponse>, CosmosGrpcError> {
        let mut grpc = StakingQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();

        let res = grpc
            .delegation(QueryDelegationRequest {
                delegator_addr: delegator.to_string(),
                validator_addr: validator.to_string(),
            })
            .await?
            .into_inner()
            .delegation_response;

        Ok(res)
    }

    /// Delegates tokens to a specified bonded validator
    pub async fn delegate_to_validator(
        &self,
        validator_address: Address,
        amount_to_delegate: Coin,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let vote = MsgDelegate {
            amount: Some(amount_to_delegate.into()),
            delegator_address: our_address.to_string(),
            validator_address: validator_address.to_string(),
        };

        let msg = Msg::new(MSG_DELEGATE_TYPE_URL, vote);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }

    /// Redelegates existing tokens without unbonding them, this operation is instant
    /// in the happy path case, but if any edge behavior is hit this will take the full
    /// unbonding time to go into effect. Examples of edge cases include redelegating twice
    /// within the unbonding period, or if many small redelegations are made in the same period
    pub async fn redelegate(
        &self,
        validator_address: Address,
        new_validator_address: Address,
        amount_to_redelegate: Coin,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let redelegate = MsgBeginRedelegate {
            amount: Some(amount_to_redelegate.into()),
            delegator_address: our_address.to_string(),
            validator_src_address: validator_address.to_string(),
            validator_dst_address: new_validator_address.to_string(),
        };

        let msg = Msg::new(MSG_BEGIN_REDELEGATE_TYPE_URL, redelegate);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }

    /// This message will start the unbonding process for the specified validator, after
    /// the unbonding period has passed these tokens will be liquid and available in the users
    /// account
    pub async fn undelegate(
        &self,
        validator_address: Address,
        amount_to_undelegate: Coin,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let undelegate = MsgUndelegate {
            amount: Some(amount_to_undelegate.into()),
            delegator_address: our_address.to_string(),
            validator_address: validator_address.to_string(),
        };

        let msg = Msg::new(MSG_UNDELEGATE_TYPE_URL, undelegate);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }
}
