//! Contains utility functions for interacting with and modifying the Cosmos sdk distribution module
//! including the community pool

use crate::error::CosmosGrpcError;
use crate::{Coin, Contact};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::query_client::QueryClient as DistQueryClient;
use cosmos_sdk_proto::cosmos::distribution::v1beta1::QueryCommunityPoolRequest;

impl Contact {
    /// Gets a list of coins in the community pool
    pub async fn get_community_pool_coins(&self) -> Result<Vec<Coin>, CosmosGrpcError> {
        let mut grpc = DistQueryClient::connect(self.url.clone()).await?;
        let res = grpc.community_pool(QueryCommunityPoolRequest {}).await?;
        let val = res.into_inner().pool;
        let mut res = Vec::new();
        for v in val {
            let parse_result = v.amount.parse();
            match parse_result {
                Ok(parse_result) => res.push(Coin {
                    denom: v.denom,
                    amount: parse_result,
                }),
                Err(e) => return Err(CosmosGrpcError::ParseError { error: e }),
            }
        }
        Ok(res)
    }
}
