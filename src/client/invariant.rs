use crate::client::msgs::MSG_VERIFY_INVARIANT_TYPE_URL;
use crate::error::CosmosGrpcError;
use crate::{Coin, Contact, Msg, PrivateKey};
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmos_sdk_proto::cosmos::crisis::v1beta1::MsgVerifyInvariant;
use cosmos_sdk_proto::cosmos::tx::v1beta1::SimulateResponse;
use std::time::Duration;

impl Contact {
    /// A utility function which simulates the specified invariant and returns the given SimulationResponse
    ///
    /// # Arguments
    /// * `module_name` - The module containing the invariant to check
    /// * `invariant_name` - The name of the invariant to check
    /// * `private_key` - A private key used to sign and send the transaction
    ///
    /// # Examples
    /// ```rust
    /// use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
    /// use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
    /// use deep_space::{Coin, client::Contact, Fee, MessageArgs, Msg, PrivateKey, CosmosPrivateKey};
    /// use std::time::Duration;
    /// let private_key = CosmosPrivateKey::from_secret("mySecret".as_bytes());
    /// let contact = Contact::new("https:://your-grpc-server", Duration::from_secs(5), "prefix").unwrap();
    /// // future must be awaited in tokio runtime
    /// contact.invariant_check("gravity", "module-balance", private_key);
    /// ```
    pub async fn invariant_check(
        &self,
        module_name: &str,
        invariant_name: &str,
        private_key: impl PrivateKey,
    ) -> Result<SimulateResponse, CosmosGrpcError> {
        trace!("Creating simulated invariant transaction");
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();

        let verify = MsgVerifyInvariant {
            sender: our_address.to_string(),
            invariant_module_name: module_name.to_string(),
            invariant_route: invariant_name.to_string(),
        };
        let msg = Msg::new(MSG_VERIFY_INVARIANT_TYPE_URL, verify);
        trace!("Submitting simulation");
        self.simulate_tx(&[msg], private_key).await
    }

    /// A utility function which executes the specified invariant and returns the TxResponse if one is given
    /// NOTE: In testing this method does not actually halt the chain due to a cosmos-sdk bug
    ///
    /// # Arguments
    /// * `module_name` - The module containing the invariant to check
    /// * `invariant_name` - The name of the invariant to check
    /// * `wait_timeout` - The amount of time to wait for the chain to respond
    /// * `private_key` - A private key used to sign and send the transaction
    ///
    /// # Examples
    /// ```rust
    /// use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
    /// use cosmos_sdk_proto::cosmos::tx::v1beta1::BroadcastMode;
    /// use deep_space::{Coin, client::Contact, Fee, MessageArgs, Msg, PrivateKey, PublicKey, CosmosPrivateKey};
    /// use std::time::Duration;
    /// let private_key = CosmosPrivateKey::from_secret("mySecret".as_bytes());
    /// let public_key = private_key.to_public_key("cosmospub").unwrap();
    /// let address = public_key.to_address();
    /// let coin = Coin {
    ///     denom: "validatortoken".to_string(),
    ///     amount: 1u32.into(),
    /// };
    /// let contact = Contact::new("https:://your-grpc-server", Duration::from_secs(5), "prefix").unwrap();
    /// // future must be awaited in tokio runtime
    /// contact.invariant_halt("gravity", "module-balance", Some(coin), Duration::from_secs(30), private_key);
    /// ```
    pub async fn invariant_halt(
        &self,
        module_name: &str,
        invariant_name: &str,
        fee_coin: Option<Coin>,
        wait_timeout: Duration,
        private_key: impl PrivateKey,
    ) -> Result<TxResponse, CosmosGrpcError> {
        trace!("Creating chain-halting invariant transaction");
        let our_address = private_key.to_address(&self.chain_prefix).unwrap();

        let verify = MsgVerifyInvariant {
            sender: our_address.to_string(),
            invariant_module_name: module_name.to_string(),
            invariant_route: invariant_name.to_string(),
        };
        let msg = Msg::new(MSG_VERIFY_INVARIANT_TYPE_URL, verify);
        trace!("Submitting chain-halting invariant");
        self.send_message(
            &[msg],
            Some("AAAAAAAHHHHHHH".to_string()),
            &[fee_coin.unwrap_or_default()],
            Some(wait_timeout),
            private_key,
        )
        .await
    }
}
