//! Contains utility functions for interacting with and submitting Cosmos governance proposals

use crate::error::CosmosGrpcError;
use crate::Address;
use crate::Coin;
use crate::Contact;
use crate::Msg;
use crate::PrivateKey;
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmos_sdk_proto::cosmos::staking::v1beta1::query_client::QueryClient as StakingQueryClient;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmos_sdk_proto::cosmos::staking::v1beta1::QueryValidatorsRequest;
use cosmos_sdk_proto::cosmos::staking::v1beta1::QueryValidatorsResponse;
use std::time::Duration;

impl Contact {
    /// Gets a list of validators
    pub async fn get_validators_list(
        &self,
        filters: QueryValidatorsRequest,
    ) -> Result<QueryValidatorsResponse, CosmosGrpcError> {
        let mut grpc = StakingQueryClient::connect(self.url.clone()).await?;
        let res = grpc.validators(filters).await?.into_inner();
        Ok(res)
    }

    /// Gets a list of bonded validators
    pub async fn get_active_validators(&self) -> Result<QueryValidatorsResponse, CosmosGrpcError> {
        let req = QueryValidatorsRequest {
            pagination: None,
            status: "BOND_STATUS_BONDED".to_string(),
        };
        self.get_validators_list(req).await
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
