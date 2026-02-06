//! Contains utilities, query endpoints, and transaction functionality for IBC transfers
//!
use crate::client::send::TransactionResponse;
use crate::client::type_urls::MSG_TRANSFER_TYPE_URL;
use crate::client::{Contact, MEMO};
use crate::coin::Coin;
use crate::error::CosmosGrpcError;
use crate::msg::Msg;
use crate::private_key::PrivateKey;
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::ibc::applications::transfer::v1::query_client::QueryClient as IbcTransferQueryClient;
use cosmos_sdk_proto::ibc::applications::transfer::v1::{
    DenomTrace, MsgTransfer, QueryDenomHashRequest, QueryDenomTraceRequest, QueryDenomTracesRequest,
};
use cosmos_sdk_proto::ibc::core::client::v1::Height;
use std::time::Duration;
use std::time::SystemTime;
use tokio::time::timeout;

impl Contact {
    /// Performs an IBC transfer, sending `amount` from the sender (derived from `private_key`)
    /// on the source chain to `receiver` on the destination chain via the specified IBC `channel_id`.
    ///
    /// # Arguments
    ///
    /// * `amount` - The coin to transfer
    /// * `fee_coin` - A fee amount and coin type to use, pass None to send a zero fee transaction
    /// * `receiver` - The bech32-encoded receiver address on the destination chain
    /// * `channel_id` - The source chain's IBC channel ID (e.g. "channel-0")
    /// * `ibc_timeout` - Duration from now for the IBC packet timeout; if the packet is not
    ///   received by the destination within this time it will be refunded
    /// * `wait_timeout` - An optional amount of time to wait for the transaction to enter the blockchain
    /// * `memo` - An optional memo to include in the IBC transfer
    /// * `private_key` - The private key used to sign and send the transaction
    ///
    /// # Examples
    /// ```rust
    /// use deep_space::{Coin, client::Contact, CosmosPrivateKey, PrivateKey};
    /// use std::time::Duration;
    /// let private_key = CosmosPrivateKey::from_secret("mySecret".as_bytes());
    /// let coin = Coin {
    ///     denom: "uatom".to_string(),
    ///     amount: 1_000_000u32.into(),
    /// };
    /// let fee = Coin {
    ///     denom: "uatom".to_string(),
    ///     amount: 5000u32.into(),
    /// };
    /// let contact = Contact::new("https://your-grpc-server:9090", Duration::from_secs(5), "cosmos").unwrap();
    /// let ibc_timeout = Duration::from_secs(60 * 10);
    /// // future must be awaited in tokio runtime
    /// contact.send_ibc_transfer(
    ///     coin,
    ///     Some(fee),
    ///     "osmo1abc...".to_string(),
    ///     "channel-0".to_string(),
    ///     ibc_timeout,
    ///     Some(Duration::from_secs(30)),
    ///     None,
    ///     private_key,
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn send_ibc_transfer(
        &self,
        amount: Coin,
        fee_coin: Option<Coin>,
        receiver: String,
        channel_id: String,
        ibc_timeout: Duration,
        wait_timeout: Option<Duration>,
        memo: Option<String>,
        private_key: impl PrivateKey,
    ) -> Result<TransactionResponse, CosmosGrpcError> {
        let sender = private_key
            .to_address(&self.chain_prefix)
            .unwrap()
            .to_string();

        let timeout_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| {
                CosmosGrpcError::BadInput("System clock error: time before UNIX_EPOCH".to_string())
            })?
            .checked_add(ibc_timeout)
            .ok_or_else(|| CosmosGrpcError::BadInput("IBC timeout duration overflow".to_string()))?
            .as_nanos()
            .try_into()
            .map_err(|_| {
                CosmosGrpcError::BadInput("Timeout timestamp exceeds u64::MAX".to_string())
            })?;

        let memo_string = memo.clone().unwrap_or_else(|| MEMO.to_string());
        let msg_transfer = MsgTransfer {
            source_port: "transfer".to_string(),
            source_channel: channel_id,
            token: Some(ProtoCoin {
                denom: amount.denom.clone(),
                amount: amount.amount.to_string(),
            }),
            sender,
            receiver,
            timeout_height: None,
            timeout_timestamp,
            memo: memo_string.clone(),
        };
        let msg = Msg::new(MSG_TRANSFER_TYPE_URL, msg_transfer);
        let fee_coins = fee_coin.map(|coin| vec![coin]).unwrap_or_default();
        self.send_message(
            &[msg],
            Some(memo_string),
            &fee_coins,
            wait_timeout,
            None,
            private_key,
        )
        .await
    }

    /// Performs an IBC transfer with explicit timeout height instead of timeout timestamp.
    ///
    /// # Arguments
    ///
    /// * `amount` - The coin to transfer
    /// * `fee_coin` - A fee amount and coin type to use, pass None to send a zero fee transaction
    /// * `receiver` - The bech32-encoded receiver address on the destination chain
    /// * `channel_id` - The source chain's IBC channel ID (e.g. "channel-0")
    /// * `timeout_height` - The block height on the destination chain after which the packet times out
    /// * `wait_timeout` - An optional amount of time to wait for the transaction to enter the blockchain
    /// * `memo` - An optional memo to include in the IBC transfer
    /// * `private_key` - The private key used to sign and send the transaction
    #[allow(clippy::too_many_arguments)]
    pub async fn send_ibc_transfer_with_height(
        &self,
        amount: Coin,
        fee_coin: Option<Coin>,
        receiver: String,
        channel_id: String,
        timeout_height: Height,
        wait_timeout: Option<Duration>,
        memo: Option<String>,
        private_key: impl PrivateKey,
    ) -> Result<TransactionResponse, CosmosGrpcError> {
        let sender = private_key
            .to_address(&self.chain_prefix)
            .unwrap()
            .to_string();

        let memo_string = memo.clone().unwrap_or_else(|| MEMO.to_string());
        let msg_transfer = MsgTransfer {
            source_port: "transfer".to_string(),
            source_channel: channel_id,
            token: Some(ProtoCoin {
                denom: amount.denom.clone(),
                amount: amount.amount.to_string(),
            }),
            sender,
            receiver,
            timeout_height: Some(timeout_height),
            timeout_timestamp: 0,
            memo: memo_string.clone(),
        };
        let msg = Msg::new(MSG_TRANSFER_TYPE_URL, msg_transfer);
        let fee_coins = fee_coin.map(|coin| vec![coin]).unwrap_or_default();
        self.send_message(
            &[msg],
            Some(memo_string),
            &fee_coins,
            wait_timeout,
            None,
            private_key,
        )
        .await
    }

    /// Queries the IBC denom trace for the given hash.
    /// Given a denom hash (e.g. the hex portion of "ibc/ABCDEF..."), this returns
    /// the full denom trace showing the transfer path and base denom.
    pub async fn query_ibc_denom_trace(
        &self,
        hash: String,
    ) -> Result<Option<DenomTrace>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            IbcTransferQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.denom_trace(QueryDenomTraceRequest { hash }),
        )
        .await??
        .into_inner();
        Ok(res.denom_trace)
    }

    /// Queries all IBC denom traces known to this chain.
    pub async fn query_ibc_denom_traces(&self) -> Result<Vec<DenomTrace>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            IbcTransferQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.denom_traces(QueryDenomTracesRequest { pagination: None }),
        )
        .await??
        .into_inner();
        Ok(res.denom_traces)
    }

    /// Queries the IBC denom hash for a given trace (e.g. "transfer/channel-0/uatom").
    /// Returns the hash that would appear in the "ibc/{hash}" denom on this chain.
    pub async fn query_ibc_denom_hash(&self, trace: String) -> Result<String, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            IbcTransferQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.denom_hash(QueryDenomHashRequest { trace }),
        )
        .await??
        .into_inner();
        Ok(res.hash)
    }
}
