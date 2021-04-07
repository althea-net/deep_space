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
    use rand::Rng;

    /// If you run the start-chains.sh script in the Gravity repo it will pass
    /// port 26657 on localhost and allow you to debug things quickly
    /// then be used to run this test and debug things quickly. You will need
    /// to run the following command and copy a phrase so that you actually
    /// have some coins to send funds
    /// docker exec -it gravity_test_instance cat /validator-phrases
    #[test]
    #[ignore]
    fn test_endpoints() {
        env_logger::init();
        let key = PrivateKey::from_phrase("destroy lock crane champion nest hurt chicken leopard field album describe glimpse chimney sort kind peanut worry dilemma anchor dismiss fox there judge arm", "").unwrap();
        let token_name = "footoken".to_string();
    }
}
