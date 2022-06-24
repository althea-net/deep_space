//! Contains utility functions for interacting with and modifying Cosmos validator staking status

use super::PAGE;
use crate::client::msgs::{MSG_SUBMIT_PROPOSAL_TYPE_URL, MSG_VOTE_TYPE_URL};
use crate::error::CosmosGrpcError;
use crate::Coin;
use crate::Contact;
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
use prost_types::Any;
use std::time::Duration;

impl Contact {
    /// Gets a list of governance proposals, user provides filter items
    pub async fn get_governance_proposals(
        &self,
        filters: QueryProposalsRequest,
    ) -> Result<QueryProposalsResponse, CosmosGrpcError> {
        let mut grpc = GovQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
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
            pagination: PAGE,
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
            pagination: PAGE,
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
            pagination: PAGE,
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
            pagination: PAGE,
        };
        self.get_governance_proposals(req).await
    }

    pub async fn vote_on_gov_proposal(
        &self,
        proposal_id: u64,
        vote: VoteOption,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let vote = MsgVote {
            proposal_id,
            voter: our_address.to_string(),
            option: vote.into(),
        };

        let msg = Msg::new(MSG_VOTE_TYPE_URL, vote);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }

    /// Provides an interface for submitting governance proposals
    pub async fn create_gov_proposal(
        &self,
        content: Any,
        deposit: Coin,
        fee: Coin,
        private_key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let proposal = MsgSubmitProposal {
            proposer: our_address.to_string(),
            content: Some(content),
            initial_deposit: vec![deposit.into()],
        };

        let msg = Msg::new(MSG_SUBMIT_PROPOSAL_TYPE_URL, proposal);
        self.send_message(&[msg], None, &[fee], wait_timeout, private_key)
            .await
    }
}
