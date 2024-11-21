//! Contains utility functions for interacting with the Cosmos sdk mint module

use crate::error::CosmosGrpcError;
use crate::Contact;
use cosmos_sdk_proto::cosmos::mint::v1beta1::query_client::QueryClient as MintQueryClient;
use cosmos_sdk_proto::cosmos::mint::v1beta1::{
    QueryAnnualProvisionsRequest, QueryInflationRequest,
};
use tokio::time::timeout;

/// When a dec is returned in the vec format and decoded as a utf8 string it will be a whole number
/// multiplied by this value to get the decimal representation
const DEC_MANTISSA: f64 = 1_000_000_000_000_000_000.0;

impl Contact {
    /// Returns the inflation rate for the chain, in decimal format
    pub async fn get_inflation(&self) -> Result<f64, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            MintQueryClient::connect(self.url.clone()),
        )
        .await??;

        let res = timeout(self.get_timeout(), grpc.inflation(QueryInflationRequest {}))
            .await??
            .into_inner();

        println!("{:?}", res);
        let string = String::from_utf8(res.inflation).unwrap();
        let float: f64 = string.parse().unwrap();
        Ok(float / DEC_MANTISSA)
    }

    /// Returns the annual provisions for the chain, in decimal format in terms of the native token per year
    pub async fn get_annual_provisions(&self) -> Result<f64, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            MintQueryClient::connect(self.url.clone()),
        )
        .await??;

        let res = timeout(
            self.get_timeout(),
            grpc.annual_provisions(QueryAnnualProvisionsRequest {}),
        )
        .await??
        .into_inner();
        println!("{:?}", res);
        let string = String::from_utf8(res.annual_provisions).unwrap();
        let float: f64 = string.parse().unwrap();
        Ok(float / DEC_MANTISSA)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn test_get_inflation() {
        let contact = Contact::new("https://gravitychain.io:9090", TIMEOUT, "gravity").unwrap();
        let result = contact.get_inflation().await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_annual_provisions() {
        let contact = Contact::new("https://gravitychain.io:9090", TIMEOUT, "gravity").unwrap();
        let result = contact.get_annual_provisions().await;
        assert!(result.is_ok());
    }
}
