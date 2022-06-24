use std::time::Duration;

pub mod bank;
pub mod distribution;
pub mod get;
pub mod gov;
pub mod invariant;
pub mod msgs;
pub mod send;
pub mod staking;
pub mod types;

use cosmos_sdk_proto::cosmos::base::query::v1beta1::PageRequest;
pub use types::ChainStatus;

use crate::{error::CosmosGrpcError, utils::ArrayString};

pub const MEMO: &str = "Sent with Deep Space";

/// The maximum number of items in a single request this is used as a stock
/// value for all pagination requests since handling pages adds complexity it's
/// simply a very large request size. So far this has not knocked anything over
pub const PAGE_SIZE: u64 = 500_000;
/// Defines a stock pagination request object for us to use in requests, comes
/// with a very large limit defined by the PAGE_SIZE constant
pub const PAGE: Option<PageRequest> = Some(PageRequest {
    key: Vec::new(),
    offset: 0,
    limit: PAGE_SIZE,
    count_total: false,
    reverse: false,
});

/// An instance of Contact Cosmos RPC Client.
#[derive(Clone)]
pub struct Contact {
    /// The GRPC server url, we connect to this address
    /// with a new instance for each call to ensure
    /// proper failover
    url: String,
    /// The maximum amount of wall time any action taken
    /// will wait for.
    timeout: Duration,
    /// The prefix being used by this node / chain for Addresses
    chain_prefix: String,
}

impl Contact {
    pub fn new(url: &str, timeout: Duration, chain_prefix: &str) -> Result<Self, CosmosGrpcError> {
        let mut url = url;
        if !url.ends_with('/') {
            url = url.trim_end_matches('/');
        }
        ArrayString::new(chain_prefix)?;
        Ok(Self {
            url: url.to_string(),
            timeout,
            chain_prefix: chain_prefix.to_string(),
        })
    }

    pub fn get_prefix(&self) -> String {
        self.chain_prefix.clone()
    }

    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::private_key::{CosmosPrivateKey, PrivateKey};
    use crate::Coin;

    const TIMEOUT: Duration = Duration::from_secs(60);

    /// If you run the start-chains.sh script in the Gravity repo it will pass
    /// port 9090 on localhost and allow you to debug things quickly
    /// then be used to run this test and debug things quickly. You will need
    /// to run the following command and copy a phrase so that you actually
    /// have some coins to send funds
    /// docker exec -it gravity_test_instance cat /validator-phrases
    #[actix_rt::test]
    #[ignore]
    async fn test_endpoints() {
        env_logger::init();
        let key = CosmosPrivateKey::from_phrase("boost casual myth skin olympic sure apology creek theme conduct view panda board pride miss turkey lonely strategy panel mad blast panda work shuffle", "").unwrap();
        let token_name = "ufootoken".to_string();
        let contact = Contact::new("http://testnet2.althea.net:9090", TIMEOUT, "althea").unwrap();
        let our_address = key.to_address(&contact.get_prefix()).unwrap();
        let destination = "althea1ezyy5y8a4pzv9jgaeh4gd2c4kmhfn4pmpjc8np"
            .parse()
            .unwrap();

        let chain_status = contact.get_chain_status().await.unwrap();
        match chain_status {
            ChainStatus::Moving { block_height: _ } => {}
            ChainStatus::Syncing | ChainStatus::WaitingToStart => panic!("Chain not running"),
        }

        let _latest_block = contact.get_latest_block().await.unwrap();
        let _account_info = contact.get_account_info(our_address).await.unwrap();

        let balances = contact.get_balances(our_address).await.unwrap();
        let mut ok = false;
        for coin in balances {
            if coin.denom == token_name {
                ok = true
            }
        }
        if !ok {
            panic!(
                "Could not find {} in the account of {}",
                token_name, our_address
            );
        }

        let send = Coin {
            denom: token_name,
            amount: 100u64.into(),
        };
        contact
            .send_coins(send.clone(), Some(send), destination, Some(TIMEOUT), key)
            .await
            .unwrap();
    }
}
