//! Contains utility functions for interacting with and modifying Cosmos validator staking status

use super::type_urls::{PARAMETER_CHANGE_PROPOSAL_TYPE_URL, SOFTWARE_UPGRADE_PROPOSAL_TYPE_URL};
use super::PAGE;
use crate::client::type_urls::{MSG_SUBMIT_PROPOSAL_TYPE_URL, MSG_VOTE_TYPE_URL};
use crate::error::CosmosGrpcError;
use crate::utils::encode_any;
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
use cosmos_sdk_proto::cosmos::params::v1beta1::ParameterChangeProposal;
use cosmos_sdk_proto::cosmos::upgrade::v1beta1::SoftwareUpgradeProposal;
use prost_types::Any;
use std::time::Duration;
use tokio::time::timeout;

#[cfg(feature = "althea")]
use super::type_urls::{REGISTER_COIN_PROPOSAL_TYPE_URL, REGISTER_ERC20_PROPOSAL_TYPE_URL};
#[cfg(feature = "althea")]
use althea_proto::canto::erc20::v1::{RegisterCoinProposal, RegisterErc20Proposal};

impl Contact {
    /// Gets a list of governance proposals, user provides filter items
    pub async fn get_governance_proposals(
        &self,
        filters: QueryProposalsRequest,
    ) -> Result<QueryProposalsResponse, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            GovQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(self.get_timeout(), grpc.proposals(filters))
            .await??
            .into_inner();
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
        self.send_message(&[msg], None, &[fee], wait_timeout, None, private_key)
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
        self.send_message(&[msg], None, &[fee], wait_timeout, None, private_key)
            .await
    }

    /// Encodes and submits a proposal to change bridge parameters
    pub async fn submit_parameter_change_proposal(
        &self,
        proposal: ParameterChangeProposal,
        deposit: Coin,
        fee: Coin,
        key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        // encode as a generic proposal
        let any = encode_any(proposal, PARAMETER_CHANGE_PROPOSAL_TYPE_URL.to_string());
        self.create_gov_proposal(any, deposit, fee, key, wait_timeout)
            .await
    }

    /// Encodes and submits a proposal to upgrade chain software
    pub async fn submit_upgrade_proposal(
        &self,
        proposal: SoftwareUpgradeProposal,
        deposit: Coin,
        fee: Coin,
        key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        // encode as a generic proposal
        let any = encode_any(proposal, SOFTWARE_UPGRADE_PROPOSAL_TYPE_URL.to_string());
        self.create_gov_proposal(any, deposit, fee, key, wait_timeout)
            .await
    }
}

#[cfg(feature = "althea")]
impl Contact {
    /// Encodes and submits a proposal to register a Coin for use with the ev module
    pub async fn submit_register_coin_proposal(
        &self,
        proposal: RegisterCoinProposal,
        deposit: Coin,
        fee: Coin,
        key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        // encode as a generic proposal
        let any = encode_any(proposal, REGISTER_COIN_PROPOSAL_TYPE_URL.to_string());
        self.create_gov_proposal(any, deposit, fee, key, wait_timeout)
            .await
    }

    /// Encodes and submits a proposal to register an ERC20 for use with the bank module
    pub async fn submit_register_erc20_proposal(
        &self,
        proposal: RegisterErc20Proposal,
        deposit: Coin,
        fee: Coin,
        key: impl PrivateKey,
        wait_timeout: Option<Duration>,
    ) -> Result<TxResponse, CosmosGrpcError> {
        // encode as a generic proposal
        let any = encode_any(proposal, REGISTER_ERC20_PROPOSAL_TYPE_URL.to_string());
        self.create_gov_proposal(any, deposit, fee, key, wait_timeout)
            .await
    }
}
