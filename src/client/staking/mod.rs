//! Contains utility functions for interacting with and submitting Cosmos governance proposals

use crate::client::MEMO;
use crate::error::CosmosGrpcError;
use crate::Address;
use crate::Coin;
use crate::Contact;
use crate::Fee;
use crate::Msg;
use crate::PrivateKey;
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmos_sdk_proto::cosmos::staking::v1beta1::query_client::QueryClient as StakingQueryClient;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmos_sdk_proto::cosmos::staking::v1beta1::QueryValidatorsRequest;
use cosmos_sdk_proto::cosmos::staking::v1beta1::QueryValidatorsResponse;
use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
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
            status: "Bonded".to_string(),
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

        let fee = Fee {
            amount: vec![fee],
            gas_limit: 500_000u64,
            granter: None,
            payer: None,
        };

        let msg = Msg::new("/cosmos.staking.v1beta1.MsgDelegate", vote);

        let args = self.get_message_args(our_address, fee).await?;
        trace!("got optional tx info");

        let msg_bytes = private_key.sign_std_msg(&[msg], args, MEMO)?;

        let response = self
            .send_transaction(msg_bytes, BroadcastMode::Sync)
            .await?;

        trace!("broadcasted! with response {:?}", response);
        if let Some(time) = wait_timeout {
            self.wait_for_tx(response, time).await
        } else {
            Ok(response)
        }
    }
}
