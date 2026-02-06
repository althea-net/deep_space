use crate::client::types::BlockParams;
use crate::client::types::*;
use crate::coin::Fee;
use crate::{address::Address, private_key::MessageArgs};
use crate::{client::Contact, error::CosmosGrpcError};
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::service_client::ServiceClient as TendermintServiceClient;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetBlockByHeightRequest;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetLatestBlockRequest;
use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::GetSyncingRequest;
use cosmos_sdk_proto::cosmos::consensus::v1::query_client::QueryClient as ConsensusQueryClient;
use cosmos_sdk_proto::cosmos::consensus::v1::QueryParamsRequest;
use cosmos_sdk_proto::cosmos::params::v1beta1::query_client::QueryClient as ParamsQueryClient;
use cosmos_sdk_proto::cosmos::params::v1beta1::{
    QueryParamsRequest as LegacyQueryParamsRequest,
    QueryParamsResponse as LegacyQueryParamsResponse,
};
use cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient as TxServiceClient;
use cosmos_sdk_proto::cosmos::tx::v1beta1::GetTxRequest;
use cosmos_sdk_proto::cosmos::tx::v1beta1::GetTxResponse;
use cosmos_sdk_proto::tendermint::types::Block;
use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;
use tokio::time::{sleep, timeout};

/// This is the default block timeout, it's used when the user doesn't specify a timeout
/// height for a transaction this will be used. It's best to always have a timeout for all transactions
/// to prevent them from becoming stuck or being included at unexpected times
pub const DEFAULT_TRANSACTION_TIMEOUT_BLOCKS: u64 = 100;

impl Contact {
    /// Gets the current chain status, returns an enum taking into account the various possible states
    /// of the chain and the requesting full node. In the common case this provides the block number
    pub async fn get_chain_status(&self) -> Result<ChainStatus, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            TendermintServiceClient::connect(self.url.clone()),
        )
        .await??;
        let syncing = timeout(self.get_timeout(), grpc.get_syncing(GetSyncingRequest {}))
            .await??
            .into_inner();
        if syncing.syncing {
            Ok(ChainStatus::Syncing)
        } else {
            let block = timeout(
                self.get_timeout(),
                grpc.get_latest_block(GetLatestBlockRequest {}),
            )
            .await?;
            match block {
                Ok(block) => match block.into_inner().block {
                    Some(block) => match block.last_commit {
                        // for some reason the block height can be negative, we cast it to a u64 for the sake
                        // of logical bounds checking
                        Some(commit) => Ok(ChainStatus::Moving {
                            block_height: u64::try_from(commit.height)?,
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
        let mut grpc = timeout(
            self.get_timeout(),
            TendermintServiceClient::connect(self.url.clone()),
        )
        .await??;
        let syncing = timeout(self.get_timeout(), grpc.get_syncing(GetSyncingRequest {}))
            .await??
            .into_inner()
            .syncing;

        let block = timeout(
            self.get_timeout(),
            grpc.get_latest_block(GetLatestBlockRequest {}),
        )
        .await?
        .unwrap();
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

    /// Gets the latest block height from the node, returns an error if no block is available
    pub async fn get_latest_block_height(&self) -> Result<u64, CosmosGrpcError> {
        let latest_block = self.get_latest_block().await?;
        match latest_block {
            LatestBlock::Latest { block } | LatestBlock::Syncing { block } => {
                if let Some(header) = block.header {
                    Ok(u64::try_from(header.height)?)
                } else {
                    Err(CosmosGrpcError::BadResponse(
                        "Null block header?".to_string(),
                    ))
                }
            }
            LatestBlock::WaitingToStart => Err(CosmosGrpcError::ChainNotRunning),
        }
    }

    /// Gets the specified block from the node, returns none if no block is available
    pub async fn get_block(&self, block: u64) -> Result<Option<Block>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            TendermintServiceClient::connect(self.url.clone()),
        )
        .await??;

        let block = timeout(
            self.get_timeout(),
            grpc.get_block_by_height(GetBlockByHeightRequest {
                height: i64::try_from(block)?,
            }),
        )
        .await??
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
        let mut grpc = timeout(
            self.get_timeout(),
            TendermintServiceClient::connect(self.url.clone()),
        )
        .await??;
        let mut result = Vec::new();
        for i in start..end {
            let block = timeout(
                self.get_timeout(),
                grpc.get_block_by_height(GetBlockByHeightRequest {
                    height: i64::try_from(i)?,
                }),
            )
            .await??
            .into_inner();
            result.push(block.block);
        }

        Ok(result)
    }

    /// Gets the specified set of blocks from the node, returning None if the block is not available
    /// this is more efficient than querying individually since it uses a single grpc session
    /// this could be made more efficient by distributing requests over several grpc sessions
    /// once some minimum size requirement was met
    pub async fn get_block_set(
        &self,
        blocks: HashSet<u64>,
    ) -> Result<Vec<Option<Block>>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            TendermintServiceClient::connect(self.url.clone()),
        )
        .await??;
        let mut result = Vec::new();
        for i in blocks {
            let block = timeout(
                self.get_timeout(),
                grpc.get_block_by_height(GetBlockByHeightRequest {
                    height: i64::try_from(i)?,
                }),
            )
            .await??
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
        let mut grpc = timeout(
            self.get_timeout(),
            ConsensusQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(self.get_timeout(), grpc.params(QueryParamsRequest {})).await?;
        if let Err(e) = res {
            if e.code() == tonic::Code::Unimplemented {
                // this means the chain doesn't have the new params query endpoint so we fall back to the old method
                debug!("Chain does not support new params query endpoint, falling back to legacy method");
                return self.get_block_params_fallback().await;
            } else {
                return Err(e.into());
            }
        }
        if let Some(v) = res.unwrap().into_inner().params {
            match v.block {
                Some(v) => Ok(BlockParams {
                    max_bytes: u64::try_from(v.max_bytes)?,
                    max_gas: Some(u64::try_from(v.max_gas)?),
                }),
                None => Err(CosmosGrpcError::BadResponse(
                    "No BlockParams? Deep Space/protos probably need an update".to_string(),
                )),
            }
        } else {
            // if we hit this error the value has been moved and we're probably
            // woefully out of date.
            Err(CosmosGrpcError::BadResponse(
                "No BlockParams? Deep Space/protos probably need an update".to_string(),
            ))
        }
    }

    async fn get_block_params_fallback(&self) -> Result<BlockParams, CosmosGrpcError> {
        // this is a fallback for chains that don't have the new params query
        // endpoint, it will be removed in the future
        #[allow(deprecated)]
        let res = self.get_param("baseapp", "BlockParams").await?;
        if let Some(v) = res.param {
            match serde_json::from_str(&v.value) {
                Ok(v) => {
                    let v: BlockParamsJson = v;
                    Ok(v.into())
                }
                Err(e) => Err(CosmosGrpcError::BadResponse(format!(
                    "Failed to parse BlockParams: {}",
                    e
                ))),
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
    #[deprecated(
        note = "Modules manage their own parameters now, use the module's grpc client to get parameters"
    )]
    pub async fn get_param(
        &self,
        subspace: impl ToString,
        key: impl ToString,
    ) -> Result<LegacyQueryParamsResponse, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            ParamsQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.params(LegacyQueryParamsRequest {
                subspace: subspace.to_string(),
                key: key.to_string(),
            }),
        )
        .await??;
        Ok(res.into_inner())
    }

    // Gets a transaction using it's hash value, TODO should fail if the transaction isn't found
    pub async fn get_tx_by_hash(&self, txhash: String) -> Result<GetTxResponse, CosmosGrpcError> {
        let mut txrpc = timeout(
            self.get_timeout(),
            TxServiceClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            txrpc.get_tx(GetTxRequest { hash: txhash }),
        )
        .await??
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
        timeout_block: Option<u64>,
    ) -> Result<MessageArgs, CosmosGrpcError> {
        let account_info = self.get_account_info(our_address).await?;
        debug!("Account info: {:?}", account_info);

        let latest_block = self.get_latest_block().await?;
        debug!("Latest block: {:?}", latest_block);

        match latest_block {
            LatestBlock::Latest { block } => {
                if let Some(header) = block.header {
                    Ok(MessageArgs {
                        sequence: account_info.sequence,
                        account_number: account_info.account_number,
                        chain_id: header.chain_id,
                        fee,
                        tip: None,
                        timeout_height: u64::try_from(header.height)?
                            + timeout_block.unwrap_or(DEFAULT_TRANSACTION_TIMEOUT_BLOCKS),
                    })
                } else {
                    Err(CosmosGrpcError::BadResponse(
                        "Null block header?".to_string(),
                    ))
                }
            }
            LatestBlock::Syncing { .. } => Err(CosmosGrpcError::NodeNotSynced),
            LatestBlock::WaitingToStart => Err(CosmosGrpcError::ChainNotRunning),
        }
    }

    /// Waits for the next block to be produced, useful if you want to wait for
    /// an on chain event or some thing to change
    pub async fn wait_for_next_block(&self, timeout: Duration) -> Result<(), CosmosGrpcError> {
        let start = Instant::now();
        let mut last_height = None;
        while Instant::now() - start < timeout {
            let res = self.get_chain_status().await;
            debug!("Got chain status: {:?}", res);
            match (res, last_height) {
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
        let max_gas = input.max_gas.parse().ok();
        let max_bytes = input.max_bytes.parse().unwrap_or(0u64);
        BlockParams { max_bytes, max_gas }
    }
}
