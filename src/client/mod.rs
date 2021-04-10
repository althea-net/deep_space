use std::time::Duration;

pub mod get;
pub mod send;
pub mod types;

pub use types::ChainStatus;

/// An instance of Contact Cosmos RPC Client.
#[derive(Clone)]
pub struct Contact {
    url: String,
    timeout: Duration,
}

impl Contact {
    pub fn new(url: &str, timeout: Duration) -> Self {
        let mut url = url;
        if !url.ends_with('/') {
            url = url.trim_end_matches('/');
        }
        Self {
            url: url.to_string(),
            timeout,
        }
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
    use crate::private_key::PrivateKey;
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
        let key = PrivateKey::from_phrase("coral earn airport scan panel burger gown fine kitten verb advice cement inform venture glass section used spin already consider cradle library option panda", "").unwrap();
        let our_address = key.to_public_key().unwrap().to_address();
        let token_name = "footoken".to_string();
        let contact = Contact::new("http://localhost:9090", TIMEOUT);
        let destination = "cosmos13lgyj4jj4vs959d8y0ytu20qufaqmhqtzqa6wj"
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
            .send_tokens(send.clone(), send, destination, key, Some(TIMEOUT))
            .await
            .unwrap();
    }
}
