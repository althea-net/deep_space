use crate::client::types::*;
use crate::coin::Coin;
use crate::coin::Fee;
use crate::{address::Address, private_key::MessageArgs};
use crate::{client::Contact, error::CosmosGrpcError};
use bytes::BytesMut;
use cosmos_sdk_proto::cosmos::auth::v1beta1::{
    query_client::QueryClient as AuthQueryClient, BaseAccount, QueryAccountRequest,
};
use cosmos_sdk_proto::cosmos::bank::v1beta1::query_client::QueryClient as BankQueryClient;
use cosmos_sdk_proto::cosmos::bank::v1beta1::QueryAllBalancesRequest;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::service_client::ServiceClient as TendermintServiceClient;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetLatestBlockRequest;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetSyncingRequest;
use cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient as TxServiceClient;
use cosmos_sdk_proto::cosmos::tx::v1beta1::GetTxRequest;
use cosmos_sdk_proto::cosmos::tx::v1beta1::GetTxResponse;
use prost::Message;
use std::time::Duration;
use std::time::Instant;
use tokio::time::sleep;

impl Contact {
    /// Gets the current chain status, returns an enum taking into account the various possible states
    /// of the chain and the requesting full node. In the common case this provides the block number
    pub async fn get_chain_status(&self) -> Result<ChainStatus, CosmosGrpcError> {
        let mut grpc = TendermintServiceClient::connect(self.url.clone()).await?;
        let syncing = grpc.get_syncing(GetSyncingRequest {}).await?.into_inner();

        if syncing.syncing {
            Ok(ChainStatus::Syncing)
        } else {
            let block = grpc.get_latest_block(GetLatestBlockRequest {}).await;
            match block {
                Ok(block) => match block.into_inner().block {
                    Some(block) => match block.last_commit {
                        // for some reason the block height can be negative, we cast it to a u64 for the sake
                        // of logical bounds checking
                        Some(commit) => Ok(ChainStatus::Moving {
                            block_height: commit.height as u64,
                        }),
                        None => Err(CosmosGrpcError::BadResponse(
                            "No commit in block?".to_string(),
                        )),
                    },
                    None => Ok(ChainStatus::WaitingToStart),
                },
                // if get syncing succeeded and this fails, it means there's 'no block' and
                // we're waiting to start
                Err(e) => {
                    if e.message().contains("nil Block") {
                        Ok(ChainStatus::WaitingToStart)
                    } else {
                        Err(e.into())
                    }
                }
            }
        }
    }

    /// Gets the latest block from the node, taking into account the possibility that the chain is halted
    /// and also the possibility that the node is syncing
    pub async fn get_latest_block(&self) -> Result<LatestBlock, CosmosGrpcError> {
        let mut grpc = TendermintServiceClient::connect(self.url.clone()).await?;
        let syncing = grpc
            .get_syncing(GetSyncingRequest {})
            .await?
            .into_inner()
            .syncing;

        let block = grpc.get_latest_block(GetLatestBlockRequest {}).await?;
        let block = block.into_inner().block;
        match block {
            Some(block) => {
                if syncing {
                    Ok(LatestBlock::Syncing { block })
                } else {
                    Ok(LatestBlock::Latest { block })
                }
            }
            None => Ok(LatestBlock::WaitingToStart),
        }
    }

    /// Gets account info for the provided Cosmos account using the accounts endpoint
    /// accounts do not have any info if they have no tokens or are otherwise never seen
    /// before in this case we return the special error NoToken
    pub async fn get_account_info(&self, address: Address) -> Result<BaseAccount, CosmosGrpcError> {
        let mut agrpc = AuthQueryClient::connect(self.url.clone()).await?;
        let res = agrpc
            // todo detect chain prefix here
            .account(QueryAccountRequest {
                address: address.to_bech32(&self.chain_prefix).unwrap(),
            })
            .await;
        match res {
            Ok(account) => {
                // null pointer if this fails to unwrap
                let value = account.into_inner().account.unwrap();
                let mut buf = BytesMut::with_capacity(value.value.len());
                buf.extend_from_slice(&value.value);
                let decoded: BaseAccount = BaseAccount::decode(buf)?;
                Ok(decoded)
            }
            Err(e) => match e.code() {
                _ => Err(CosmosGrpcError::RequestError { error: e }),
            },
        }
    }

    // Gets a transaction using it's hash value, TODO should fail if the transaction isn't found
    pub async fn get_tx_by_hash(&self, txhash: String) -> Result<GetTxResponse, CosmosGrpcError> {
        let mut txrpc = TxServiceClient::connect(self.url.clone()).await?;
        let res = txrpc
            .get_tx(GetTxRequest { hash: txhash })
            .await?
            .into_inner();
        Ok(res)
    }

    pub async fn get_balances(&self, address: Address) -> Result<Vec<Coin>, CosmosGrpcError> {
        let mut bankrpc = BankQueryClient::connect(self.url.clone()).await?;
        let res = bankrpc
            .all_balances(QueryAllBalancesRequest {
                // chain prefix is validated as part of this client, so this can't
                // panic
                address: address.to_bech32(&self.chain_prefix).unwrap(),
                pagination: None,
            })
            .await?
            .into_inner();
        let balances = res.balances;
        let mut ret = Vec::new();
        for value in balances {
            ret.push(value.into());
        }
        Ok(ret)
    }

    /// Grabs an up to date MessageArgs structure for an address,
    /// provided a fee value to insert into the structure. The goal of
    /// this function is to be very minimal and make a lot of choices for
    /// the user. Like how to handle changes in chain-id or timeout heights
    pub async fn get_message_args(
        &self,
        our_address: Address,
        fee: Fee,
    ) -> Result<MessageArgs, CosmosGrpcError> {
        let account_info = self.get_account_info(our_address).await?;
        let latest_block = self.get_latest_block().await?;

        match latest_block {
            LatestBlock::Latest { block } => {
                if let Some(header) = block.header {
                    Ok(MessageArgs {
                        sequence: account_info.sequence,
                        account_number: account_info.account_number,
                        chain_id: header.chain_id,
                        fee,
                        timeout_height: header.height as u64 + 100,
                    })
                } else {
                    Err(CosmosGrpcError::BadResponse(
                        "Null block header?".to_string(),
                    ))
                }
            }
            LatestBlock::Syncing { .. } => Err(CosmosGrpcError::NodeNotSynced),
            LatestBlock::WaitingToStart { .. } => Err(CosmosGrpcError::ChainNotRunning),
        }
    }

    /// Waits for the next block to be produced, useful if you want to wait for
    /// an on chain event or some thing to change
    pub async fn wait_for_next_block(&self, timeout: Duration) -> Result<(), CosmosGrpcError> {
        let start = Instant::now();
        let mut last_height = None;
        while Instant::now() - start < timeout {
            match (self.get_chain_status().await, last_height) {
                (Ok(ChainStatus::Moving { block_height }), None) => {
                    last_height = Some(block_height)
                }
                (Ok(ChainStatus::Moving { block_height }), Some(last_height)) => {
                    if block_height > last_height {
                        return Ok(());
                    }
                }
                (Ok(ChainStatus::Syncing), _) => return Err(CosmosGrpcError::NodeNotSynced),
                (Ok(ChainStatus::WaitingToStart), _) => {
                    return Err(CosmosGrpcError::ChainNotRunning)
                }
                // we don't want a single error to exit this loop early
                (Err(_), _) => {}
            }
            sleep(Duration::from_secs(1)).await;
        }
        Err(CosmosGrpcError::NoBlockProduced { time: timeout })
    }
}
