//! Contains utility functions for interacting with and submitting Cosmos governance proposals

use crate::error::CosmosGrpcError;
use crate::Address;
use crate::Coin;
use crate::Contact;
use crate::Msg;
use crate::PrivateKey;
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmos_sdk_proto::cosmos::staking::v1beta1::query_client::QueryClient as StakingQueryClient;
use cosmos_sdk_proto::cosmos::staking::v1beta1::DelegationResponse;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
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
            pagination: None,
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
                pagination: None,
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
        private_key: PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let vote = MsgDelegate {
            amount: Some(amount_to_delegate.into()),
            delegator_address: our_address.to_string(),
            validator_address: validator_address.to_string(),
        };

        let msg = Msg::new("/cosmos.staking.v1beta1.MsgDelegate", vote);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }
}
