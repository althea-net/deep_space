use crate::address::Address;
#[cfg(feature = "althea")]
use crate::client::type_urls::MSG_MICROTX_TYPE_URL;
use crate::client::type_urls::MSG_SEND_TYPE_URL;
use crate::client::Contact;
use crate::client::MEMO;
use crate::coin::Coin;
use crate::coin::Fee;
use crate::error::CosmosGrpcError;
use crate::msg::Msg;
use crate::private_key::PrivateKey;
use crate::utils::check_for_sdk_error;
use crate::MessageArgs;
#[cfg(feature = "althea")]
use althea_proto::althea::microtx::v1::MsgMicrotx;
use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastTxRequest;
use cosmos_sdk_proto::cosmos::tx::v1beta1::SimulateRequest;
use cosmos_sdk_proto::cosmos::tx::v1beta1::SimulateResponse;
use cosmos_sdk_proto::cosmos::{
    base::abci::v1beta1::TxResponse, tx::v1beta1::service_client::ServiceClient as TxServiceClient,
};
use std::time::Instant;
use std::{clone::Clone, time::Duration};
use tokio::time::sleep;
use tokio::time::timeout;
use tonic::Code as TonicCode;

impl Contact {
    /// Sends an already serialized and signed transaction, checking for various errors in the
    /// transaction response. This is the lowest level transaction sending function and you
    /// probably shouldn't use it unless you have specific needs. `send_message` is more
    /// appropriate for general use.
    ///
    /// # Arguments
    ///
    /// * `msg` - A proto encoded and already signed message in byte format
    /// * `mode` - The Broadcast mode to use, `BroadcastMode::Sync` waits for basic validation
    ///            `BroadcastMode::Block` is supposed to wait for the tx to enter the chain
    ///            but grpc timeouts mean this is unreliable. `BroadcastMode::Async` sends and
    ///            returns without waiting for any validation
    /// # Examples
    /// ```rust
    /// use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
    /// use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
    /// use deep_space::{Coin, client::Contact, Fee, MessageArgs, Msg, CosmosPrivateKey, PrivateKey, PublicKey};
    /// use deep_space::client::type_urls::SECP256K1_PUBKEY_TYPE_URL;
    /// use std::time::Duration;
    /// let private_key = CosmosPrivateKey::from_secret("mySecret".as_bytes());
    /// let public_key = private_key.to_public_key("cosmospub").unwrap();
    /// let address = public_key.to_address();
    /// let coin = Coin {
    ///     denom: "validatortoken".to_string(),
    ///     amount: 1u32.into(),
    /// };
    /// let send = MsgSend {
    ///     amount: vec![coin.clone().into()],
    ///     from_address: address.to_string(),
    ///     to_address: "cosmos1pr2n6tfymnn2tk6rkxlu9q5q2zq5ka3wtu7sdj".to_string(),
    /// };
    /// let fee = Fee {
    ///     amount: vec![coin],
    ///     gas_limit: 500_000,
    ///     granter: None,
    ///     payer: None,
    /// };
    /// let msg = Msg::new(SECP256K1_PUBKEY_TYPE_URL, send);
    /// let args = MessageArgs {
    ///     sequence: 0,
    ///     account_number: 0,
    ///     chain_id: "mychainid".to_string(),
    ///     fee,
    ///     timeout_height: 100,
    /// };
    /// let tx = private_key.sign_std_msg(&[msg], args, "").unwrap();
    /// let contact = Contact::new("https:://your-grpc-server", Duration::from_secs(5), "prefix").unwrap();
    /// // future must be awaited in tokio runtime
    /// contact.send_transaction(tx, BroadcastMode::Sync);
    /// ```
    pub async fn send_transaction(
        &self,
        // proto serialized message for us to turn into an 'any' object
        msg: Vec<u8>,
        mode: BroadcastMode,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let mut txrpc =
            timeout(self.get_timeout(), TxServiceClient::connect(self.get_url())).await??;
        let response = timeout(
            self.get_timeout(),
            txrpc.broadcast_tx(BroadcastTxRequest {
                tx_bytes: msg,
                mode: mode.into(),
            }),
        )
        .await??;
        let response = response.into_inner().tx_response.unwrap();
        // checks only for sdk errors, other types will not be handled
        check_for_sdk_error(&response)?;
        Ok(response)
    }

    /// High level message sending function, you provide an arbitrary vector of messages to send
    /// a private key to sign with, and a fee coin (if any). The gas is then estimated and set
    /// automatically. This function will return on or before the provided wait_timeout value
    /// if no timeout is provided we will still wait for a response from the Cosmos node with
    /// the results of ValidateBasic() on your transaction this may take up to a few seconds.
    ///
    /// # Arguments
    ///
    /// * `messages` - An array of messages to send
    /// * `memo` - An optional memo to be included in the transaction, if None the default memo value is set
    /// * `fee_coin` - A fee amount and coin type to use, pass an empty array to send a zero fee transaction
    /// * `wait_timeout` - An optional amount of time to wait for the transaction to enter the blockchain
    /// * `block_timeout` - An optional number of blocks into the future that this transaction should be valid for. 
    ///                     If None, DEFAULT_TRANSACTION_TIMEOUT_BLOCKS is used.
    /// * `private_key` - A private key used to sign and send the transaction
    /// # Examples
    /// ```rust
    /// use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
    /// use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
    /// use deep_space::{Coin, client::Contact, Fee, MessageArgs, Msg, CosmosPrivateKey, PrivateKey, PublicKey};
    /// use std::time::Duration;
    /// use deep_space::client::type_urls::SECP256K1_PUBKEY_TYPE_URL;
    /// let private_key = CosmosPrivateKey::from_secret("mySecret".as_bytes());
    /// let public_key = private_key.to_public_key("cosmospub").unwrap();
    /// let address = public_key.to_address();
    /// let coin = Coin {
    ///     denom: "validatortoken".to_string(),
    ///     amount: 1u32.into(),
    /// };
    /// let send = MsgSend {
    ///     amount: vec![coin.clone().into()],
    ///     from_address: address.to_string(),
    ///     to_address: "cosmos1pr2n6tfymnn2tk6rkxlu9q5q2zq5ka3wtu7sdj".to_string(),
    /// };
    /// let msg = Msg::new(SECP256K1_PUBKEY_TYPE_URL, send);
    /// let contact = Contact::new("https:://your-grpc-server", Duration::from_secs(5), "prefix").unwrap();
    /// // future must be awaited in tokio runtime
    /// contact.send_message(&vec![msg], None, &[coin], None, private_key);
    /// ```
    pub async fn send_message(
        &self,
        messages: &[Msg],
        memo: Option<String>,
        fee_coin: &[Coin],
        wait_timeout: Option<Duration>,
        block_timeout: Option<u64>,
        private_key: impl PrivateKey,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();

        let fee = self
            .get_fee_info(messages, fee_coin, private_key.clone())
            .await?;
        let args = self.get_message_args(our_address, fee, block_timeout).await?;
        trace!("got optional tx info");

        self.send_message_with_args(messages, memo, args, wait_timeout, private_key)
            .await
    }

    /// Performs Tx generation, signing, and submission for send_message()
    /// See send_message() for more information
    ///
    /// This method is particularly useful for queuing messages for sequential submission
    pub async fn send_message_with_args(
        &self,
        messages: &[Msg],
        memo: Option<String>,
        args: MessageArgs,
        wait_timeout: Option<Duration>,
        private_key: impl PrivateKey,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let memo = memo.unwrap_or_else(|| MEMO.to_string());
        let msg_bytes = private_key.sign_std_msg(messages, args, &memo)?;

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

    /// Simulates the provided array of messages and returns
    /// a fee object with the gas amount actually used
    pub async fn get_fee_info(
        &self,
        messages: &[Msg],
        fee_token: &[Coin],
        private_key: impl PrivateKey,
    ) -> Result<Fee, CosmosGrpcError> {
        let gas_info = self
            .simulate_tx(messages, Some(fee_token), private_key.clone())
            .await?
            .gas_info
            .unwrap();
        let gas_used = gas_info.gas_used;
        trace!("Got {} gas used!", gas_used);

        let block_params = self.get_block_params().await?;
        if let Some(max_gas) = block_params.max_gas {
            if gas_used > max_gas {
                return Err(CosmosGrpcError::GasRequiredExceedsBlockMaximum {
                    max: max_gas,
                    required: gas_used,
                });
            }

            // check if max gas and gas used are close by seeing
            // if we can divide max_gas by gas used, a value of one
            // indicates that it's more than half
            if let Some(m) = max_gas.checked_div(gas_used) {
                if m == 1 {
                    warn!(
                        "Tx simulation has gas usage {} which is close to max_gas {}. \n
                        Gas estimation is known to be inaccurate! When you submit a tx that \n
                        requires more than the block max gas, you will not get an error message! \n
                        Just an unexplained timeout. Watch for this.",
                        gas_used, max_gas
                    )
                }
            }
        }

        Ok(Fee {
            amount: fee_token.to_vec(),
            granter: None,
            payer: None,
            // due to this known issue, gas estimation is
            // inaccurate, normally short about ~20% in my tests
            // https://github.com/cosmos/cosmos-sdk/issues/4938
            gas_limit: gas_used * 2,
        })
    }

    /// Simulates the provided array of messages and returns
    /// the simulation result
    pub async fn simulate_tx(
        &self,
        messages: &[Msg],
        fee_amount: Option<&[Coin]>,
        private_key: impl PrivateKey,
    ) -> Result<SimulateResponse, CosmosGrpcError> {
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();
        let fee_amount = fee_amount.unwrap_or_default();
        let mut txrpc =
            timeout(self.get_timeout(), TxServiceClient::connect(self.get_url())).await??;

        let fee_obj = Fee {
            amount: fee_amount.to_vec(),
            // derived from this constant https://github.com/cosmos/cosmos-sdk/blob/master/types/tx/types.go#L13
            gas_limit: 9223372036854775807,
            granter: None,
            payer: None,
        };

        let args = self.get_message_args(our_address, fee_obj, None).await?;

        let tx_bytes = private_key.sign_std_msg(messages, args, MEMO)?;

        // used to avoid the deprication warning on SimulateRequest
        #[allow(deprecated)]
        let sim_request = SimulateRequest { tx_bytes, tx: None };

        let response = timeout(self.get_timeout(), txrpc.simulate(sim_request)).await??;
        let response = response.into_inner();

        Ok(response)
    }

    /// A utility function that creates a one to one simple Coin transfer
    /// and sends it from the provided private key, waiting the configured
    /// amount of time for the tx to enter the chain, if you do not specify
    /// a fee the smallest working amount will be selected.
    ///
    /// # Arguments
    ///
    /// * `coin` - The amount and type of coin you are sending
    /// * `fee_coin` - A fee amount and coin type to use, pass None to send a zero fee transaction
    /// * `destination` - The target destination address
    /// * `wait_timeout` - An optional amount of time to wait for the transaction to enter the blockchain
    /// * `private_key` - A private key used to sign and send the transaction
    /// # Examples
    /// ```rust
    /// use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
    /// use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
    /// use deep_space::{Coin, client::Contact, Fee, MessageArgs, Msg, CosmosPrivateKey, PrivateKey, PublicKey};
    /// use std::time::Duration;
    /// let private_key = CosmosPrivateKey::from_secret("mySecret".as_bytes());
    /// let public_key = private_key.to_public_key("cosmospub").unwrap();
    /// let address = public_key.to_address();
    /// let coin = Coin {
    ///     denom: "validatortoken".to_string(),
    ///     amount: 1000000u32.into(),
    /// };
    /// let fee = Coin {
    ///     denom: "validatortoken".to_string(),
    ///     amount: 1u32.into(),
    /// };
    /// let contact = Contact::new("https:://your-grpc-server", Duration::from_secs(5), "prefix").unwrap();
    /// let duration = Duration::from_secs(30);
    /// // future must be awaited in tokio runtime
    /// contact.send_coins(coin.clone(), Some(fee), address, Some(duration), private_key);
    /// ```
    pub async fn send_coins(
        &self,
        coin: Coin,
        fee_coin: Option<Coin>,
        destination: Address,
        wait_timeout: Option<Duration>,
        private_key: impl PrivateKey,
    ) -> Result<TxResponse, CosmosGrpcError> {
        trace!("Creating transaction");
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();

        let send = MsgSend {
            amount: vec![coin.into()],
            from_address: our_address.to_bech32(&self.chain_prefix).unwrap(),
            to_address: destination.to_bech32(&self.chain_prefix).unwrap(),
        };
        let msg = Msg::new(MSG_SEND_TYPE_URL, send);
        self.send_message(
            &[msg],
            None,
            &[fee_coin.unwrap_or_default()],
            wait_timeout,
            None,
            private_key,
        )
        .await
    }

    #[cfg(feature = "althea")]
    /// A utility function that executes a microtransaction on the Althea Chain, meant to be used by routers
    /// on Althea networks to pay peers for internet service.
    ///
    /// # Arguments
    ///
    /// * `coin` - The amount and type of coin you are sending
    /// * `fee_coin` - A fee amount and coin type to use, pass None to send a zero fee transaction
    /// * `destination` - The target destination address
    /// * `wait_timeout` - An optional amount of time to wait for the transaction to enter the blockchain
    /// * `block_timeout` - A time period in blocks from when this tx is sent that it will be valid for.
    ///                     The default value is DEFAULT_TRANSACTION_TIMEOUT_BLOCKS
    /// * `private_key` - A private key used to sign and send the transaction
    /// # Examples
    /// ```rust
    /// use althea_proto::microtx::v1::MsgMicrotx;
    /// use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
    /// use deep_space::{Coin, client::Contact, Fee, MessageArgs, Msg, CosmosPrivateKey, PrivateKey, PublicKey};
    /// use std::time::Duration;
    /// let private_key = CosmosPrivateKey::from_secret("mySecret".as_bytes());
    /// let public_key = private_key.to_public_key("cosmospub").unwrap();
    /// let address = public_key.to_address();
    /// let coin = Coin {
    ///     denom: "validatortoken".to_string(),
    ///     amount: 1000000u32.into(),
    /// };
    /// let fee = Coin {
    ///     denom: "validatortoken".to_string(),
    ///     amount: 10u32.into(),
    /// };
    /// let contact = Contact::new("https:://your-grpc-server", Duration::from_secs(5), "prefix").unwrap();
    /// let duration = Duration::from_secs(30);
    /// // future must be awaited in tokio runtime
    /// contact.send_microtx(coin.clone(), Some(fee), address, Some(duration), private_key);
    /// ```
    pub async fn send_microtx(
        &self,
        coin: Coin,
        fee_coin: Option<Coin>,
        destination: Address,
        wait_timeout: Option<Duration>,
        block_timeout: Option<u64>,
        private_key: impl PrivateKey,
    ) -> Result<TxResponse, CosmosGrpcError> {
        trace!("Creating transaction");
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();

        let send = MsgMicrotx {
            sender: our_address.to_bech32(&self.chain_prefix).unwrap(),
            receiver: destination.to_bech32(&self.chain_prefix).unwrap(),
            amount: Some(coin.into()),
        };
        let msg = Msg::new(MSG_MICROTX_TYPE_URL, send);
        self.send_message(
            &[msg],
            None,
            &[fee_coin.unwrap_or_default()],
            wait_timeout,
            block_timeout,
            private_key,
        )
        .await
    }

    /// Utility function that waits for a tx to enter the chain by querying
    /// it's txid, will not exit for timeout time unless the error is known
    /// and unrecoverable
    pub async fn wait_for_tx(
        &self,
        response: TxResponse,
        timeout: Duration,
    ) -> Result<TxResponse, CosmosGrpcError> {
        let start = Instant::now();
        while Instant::now() - start < timeout {
            // TODO what actually determines when the tx is in the chain?
            let status = self.get_tx_by_hash(response.txhash.clone()).await;
            match status {
                Ok(status) => {
                    if let Some(res) = status.tx_response {
                        return Ok(res);
                    }
                }
                Err(CosmosGrpcError::RequestError { error }) => match error.code() {
                    TonicCode::NotFound | TonicCode::Unknown | TonicCode::InvalidArgument => {}
                    _ => {
                        return Err(CosmosGrpcError::TransactionFailed {
                            tx: response,
                            time: Instant::now() - start,
                            sdk_error: None,
                        });
                    }
                },
                Err(e) => return Err(e),
            }
            sleep(Duration::from_secs(1)).await;
        }
        Err(CosmosGrpcError::TransactionFailed {
            tx: response,
            time: timeout,
            sdk_error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::CosmosPrivateKey;

    use super::*;

    const TIMEOUT: Duration = Duration::from_secs(60);

    #[ignore]
    #[actix_rt::test]
    async fn test_send_to_althea() {
        env_logger::init();
        let contact = Contact::new("http://localhost:9090", TIMEOUT, "althea").unwrap();
        let mnemonic = "prepare meadow assault rifle biology animal visit eight purchase dinosaur question question inside sister ignore any airport tell ecology extend dove wrist mean comfort";
        let private_key: CosmosPrivateKey = PrivateKey::from_phrase(mnemonic, "").unwrap();
        let coin = Coin {
            denom: "aalthea".to_string(),
            amount: 100u32.into(),
        };
        let destination =
            Address::from_bech32("althea1nldz7ns4hn4waar3e90mafdyucqts5xzemg28d".to_string())
                .unwrap();
        let fee = Coin {
            denom: "aalthea".to_string(),
            amount: 100u32.into(),
        };

        let res = contact
            .send_coins(coin, Some(fee), destination, Some(TIMEOUT), private_key)
            .await;
        assert!(res.is_ok())
    }
}
