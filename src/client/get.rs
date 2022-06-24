use crate::client::types::BaseAccount;
use crate::client::types::BlockParams;
use crate::client::types::CosmosAccount;
use crate::client::types::*;
use crate::coin::Fee;
use crate::{address::Address, private_key::MessageArgs};
use crate::{client::Contact, error::CosmosGrpcError};
use bytes::BytesMut;
use cosmos_sdk_proto::cosmos::auth::v1beta1::{
    query_client::QueryClient as AuthQueryClient, BaseAccount as ProtoBaseAccount,
    QueryAccountRequest,
};
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::service_client::ServiceClient as TendermintServiceClient;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetBlockByHeightRequest;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetLatestBlockRequest;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetSyncingRequest;
use cosmos_sdk_proto::cosmos::params::v1beta1::query_client::QueryClient as ParamsQueryClient;
use cosmos_sdk_proto::cosmos::params::v1beta1::QueryParamsRequest;
use cosmos_sdk_proto::cosmos::params::v1beta1::QueryParamsResponse;
use cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient as TxServiceClient;
use cosmos_sdk_proto::cosmos::tx::v1beta1::GetTxRequest;
use cosmos_sdk_proto::cosmos::tx::v1beta1::GetTxResponse;
use cosmos_sdk_proto::cosmos::vesting::v1beta1::ContinuousVestingAccount;
use cosmos_sdk_proto::cosmos::vesting::v1beta1::DelayedVestingAccount;
use cosmos_sdk_proto::cosmos::vesting::v1beta1::PeriodicVestingAccount;
use cosmos_sdk_proto::tendermint::types::Block;
use prost::Message;
use std::time::Duration;
use std::time::Instant;
use tokio::time::sleep;
use tonic::Code as GrpcCode;

impl Contact {
    /// Gets the current chain status, returns an enum taking into account the various possible states
    /// of the chain and the requesting full node. In the common case this provides the block number
    pub async fn get_chain_status(&self) -> Result<ChainStatus, CosmosGrpcError> {
        let mut grpc = TendermintServiceClient::connect(self.url.clone())
            .await?
            .accept_gzip();
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
        let mut grpc = TendermintServiceClient::connect(self.url.clone())
            .await?
            .accept_gzip();
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

    /// Gets the specified block from the node, returns none if no block is available
    pub async fn get_block(&self, block: u64) -> Result<Option<Block>, CosmosGrpcError> {
        let mut grpc = TendermintServiceClient::connect(self.url.clone())
            .await?
            .accept_gzip();

        let block = grpc
            .get_block_by_height(GetBlockByHeightRequest {
                height: block as i64,
            })
            .await?
            .into_inner();
        Ok(block.block)
    }

    /// Gets the specified block range from the node, returning None if no block is available
    /// this is more efficient than querying individually since it uses a single grpc session
    /// this could be made more efficient by distributing requests over several grpc sessions
    /// once some minimum range requirement was met
    pub async fn get_block_range(
        &self,
        start: u64,
        end: u64,
    ) -> Result<Vec<Option<Block>>, CosmosGrpcError> {
        let mut grpc = TendermintServiceClient::connect(self.url.clone())
            .await?
            .accept_gzip();

        let mut result = Vec::new();
        for i in start..end {
            let block = grpc
                .get_block_by_height(GetBlockByHeightRequest { height: i as i64 })
                .await?
                .into_inner();
            result.push(block.block);
        }

        Ok(result)
    }

    /// Queries the block params, including max block tx size and gas from the chain, useful for
    /// determining just how big a transaction can be before it will be rejected.
    /// This is extra useful because cosmos-sdk behaves very strangely when
    /// a transaction above the max allowed gas is submitted.
    pub async fn get_block_params(&self) -> Result<BlockParams, CosmosGrpcError> {
        let res = self.get_param("baseapp", "BlockParams").await?;
        if let Some(v) = res.param {
            match serde_json::from_str(&v.value) {
                Ok(v) => {
                    let v: BlockParamsJson = v;
                    Ok(v.into())
                }
                Err(e) => Err(CosmosGrpcError::BadResponse(e.to_string())),
            }
        } else {
            // if we hit this error the value has been moved and we're probably
            // woefully out of date.
            Err(CosmosGrpcError::BadResponse(
                "No BlockParams? Deep Space probably needs to be upgraded".to_string(),
            ))
        }
    }

    /// Queries a registered parameter given it's subspace and key, this should work
    /// for any module so long as it has registered the parameter
    pub async fn get_param(
        &self,
        subspace: impl ToString,
        key: impl ToString,
    ) -> Result<QueryParamsResponse, CosmosGrpcError> {
        let mut grpc = ParamsQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        Ok(grpc
            .params(QueryParamsRequest {
                subspace: subspace.to_string(),
                key: key.to_string(),
            })
            .await?
            .into_inner())
    }

    /// Gets account info for the provided Cosmos account using the accounts endpoint
    /// accounts do not have any info if they have no tokens or are otherwise never seen
    /// before in this case we return the special error NoToken
    pub async fn get_account_info(&self, address: Address) -> Result<BaseAccount, CosmosGrpcError> {
        let mut agrpc = AuthQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let query = QueryAccountRequest {
            address: address.to_bech32(&self.chain_prefix).unwrap(),
        };
        let res = agrpc
            // todo detect chain prefix here
            .account(query)
            .await;
        match res {
            Ok(account) => {
                // null pointer if this fails to unwrap
                let value = account.into_inner().account.unwrap();
                let mut buf = BytesMut::with_capacity(value.value.len());
                buf.extend_from_slice(&value.value);
                match (
                    ProtoBaseAccount::decode(buf.clone()),
                    PeriodicVestingAccount::decode(buf.clone()),
                    ContinuousVestingAccount::decode(buf.clone()),
                    DelayedVestingAccount::decode(buf.clone()),
                ) {
                    (Ok(d), _, _, _) => Ok(d.get_base_account()),
                    (_, Ok(d), _, _) => Ok(d.get_base_account()),
                    (_, _, Ok(d), _) => Ok(d.get_base_account()),
                    (_, _, _, Ok(d)) => Ok(d.get_base_account()),
                    (Err(e), _, _, _) => Err(CosmosGrpcError::DecodeError { error: e }),
                }
            }
            Err(e) => match e.code() {
                GrpcCode::NotFound => Err(CosmosGrpcError::NoToken),
                _ => Err(CosmosGrpcError::RequestError { error: e }),
            },
        }
    }

    // Gets a transaction using it's hash value, TODO should fail if the transaction isn't found
    pub async fn get_tx_by_hash(&self, txhash: String) -> Result<GetTxResponse, CosmosGrpcError> {
        let mut txrpc = TxServiceClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let res = txrpc
            .get_tx(GetTxRequest { hash: txhash })
            .await?
            .into_inner();
        Ok(res)
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

/// One off struct for deserialization of the BlockParams struct
#[derive(Serialize, Deserialize, Debug, Clone)]
struct BlockParamsJson {
    max_bytes: String,
    max_gas: String,
}
impl From<BlockParamsJson> for BlockParams {
    fn from(input: BlockParamsJson) -> Self {
        let max_gas = match input.max_gas.parse() {
            Ok(v) => Some(v),
            Err(_) => None,
        };
        let max_bytes = input.max_bytes.parse().unwrap_or(0u64);
        BlockParams { max_bytes, max_gas }
    }
}
