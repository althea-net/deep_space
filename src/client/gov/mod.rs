//! Contains utility functions for interacting with and modifying Cosmos validator staking status

use crate::client::MEMO;
use crate::error::CosmosGrpcError;
use crate::Coin;
use crate::Contact;
use crate::Fee;
use crate::Msg;
use crate::PrivateKey;
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmos_sdk_proto::cosmos::gov::v1beta1::query_client::QueryClient as GovQueryClient;
use cosmos_sdk_proto::cosmos::gov::v1beta1::MsgSubmitProposal;
use cosmos_sdk_proto::cosmos::gov::v1beta1::MsgVote;
use cosmos_sdk_proto::cosmos::gov::v1beta1::ProposalStatus;
use cosmos_sdk_proto::cosmos::gov::v1beta1::QueryProposalsRequest;
use cosmos_sdk_proto::cosmos::gov::v1beta1::QueryProposalsResponse;
use cosmos_sdk_proto::cosmos::gov::v1beta1::VoteOption;
use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
use prost_types::Any;
use std::time::Duration;

impl Contact {
    /// Gets a list of governance proposals, user provides filter items
    pub async fn get_governance_proposals(
        &self,
        filters: QueryProposalsRequest,
    ) -> Result<QueryProposalsResponse, CosmosGrpcError> {
        let mut grpc = GovQueryClient::connect(self.url.clone()).await?;
        let res = grpc.proposals(filters).await?.into_inner();
        Ok(res)
    }

    /// Gets a list of all active governance proposals currently in the voting period
    pub async fn get_governance_proposals_in_voting_period(
        &self,
    ) -> Result<QueryProposalsResponse, CosmosGrpcError> {
        let req = QueryProposalsRequest {
            // Go default values indicate that this search param is not
            // being used
            depositor: String::new(),
            proposal_status: ProposalStatus::VotingPeriod.into(),
            voter: String::new(),
            pagination: None,
        };
        self.get_governance_proposals(req).await
    }

    /// Gets a list of all governance proposals that have passed
    pub async fn get_passed_governance_proposals(
        &self,
    ) -> Result<QueryProposalsResponse, CosmosGrpcError> {
        let req = QueryProposalsRequest {
            // Go default values indicate that this search param is not
            // being used
            depositor: String::new(),
            proposal_status: ProposalStatus::Passed.into(),
            voter: String::new(),
            pagination: None,
        };
        self.get_governance_proposals(req).await
    }

    /// Gets a list of all governance proposals that have failed
    pub async fn get_failed_governance_proposals(
        &self,
    ) -> Result<QueryProposalsResponse, CosmosGrpcError> {
        let req = QueryProposalsRequest {
            // Go default values indicate that this search param is not
            // being used
            depositor: String::new(),
            proposal_status: ProposalStatus::Failed.into(),
            voter: String::new(),
            pagination: None,
        };
        self.get_governance_proposals(req).await
    }

    /// Gets a list of all governance proposals that have been rejected
    pub async fn get_rejected_governance_proposals(
        &self,
    ) -> Result<QueryProposalsResponse, CosmosGrpcError> {
        let req = QueryProposalsRequest {
            // Go default values indicate that this search param is not
            // being used
            depositor: String::new(),
            proposal_status: ProposalStatus::Rejected.into(),
            voter: String::new(),
            pagination: None,
        };
        self.get_governance_proposals(req).await
    }

    pub async fn vote_on_gov_proposal(
        &self,
        proposal_id: u64,
        vote: VoteOption,
        fee: Coin,
        private_key: PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let vote = MsgVote {
            proposal_id,
            voter: our_address.to_string(),
            option: vote.into(),
        };

        let fee = Fee {
            amount: vec![fee],
            gas_limit: 500_000u64,
            granter: None,
            payer: None,
        };

        let msg = Msg::new("/cosmos.gov.v1betav1.MsgVote", vote);

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

    /// Provides an interface for submitting governance proposals
    pub async fn create_gov_proposal(
        &self,
        content: Any,
        deposit: Coin,
        fee: Coin,
        private_key: PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let proposal = MsgSubmitProposal {
            proposer: our_address.to_string(),
            content: Some(content),
            initial_deposit: vec![deposit.into()],
        };

        let fee = Fee {
            amount: vec![fee],
            gas_limit: 500_000u64,
            granter: None,
            payer: None,
        };

        let msg = Msg::new("/cosmos.gov.v1betav1.MsgSubmitProposal", proposal);

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
