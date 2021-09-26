//! Contains utility functions for interacting with and modifying the Cosmos sdk distribution module
//! including the community pool

use crate::error::CosmosGrpcError;
use crate::{Coin, Contact};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::query_client::QueryClient as DistQueryClient;
use cosmos_sdk_proto::cosmos::distribution::v1beta1::QueryCommunityPoolRequest;
use num256::Uint256;
use num_bigint::ParseBigIntError;

// required because dec coins are multiplied by 1*10^18
const ONE_ETH: u128 = 10u128.pow(18);

impl Contact {
    /// Gets a list of coins in the community pool, note returned values from this endpoint
    /// are in DecCoins for precision, for the sake of ease of use this endpoint converts them
    /// into their normal form, for easy comparison against any other coin or amount.
    pub async fn get_community_pool_coins(&self) -> Result<Vec<Coin>, CosmosGrpcError> {
        let mut grpc = DistQueryClient::connect(self.url.clone()).await?;
        let res = grpc.community_pool(QueryCommunityPoolRequest {}).await?;
        let val = res.into_inner().pool;
        let mut res = Vec::new();
        for v in val {
            let parse_result: Result<Uint256, ParseBigIntError> = v.amount.parse();
            match parse_result {
                Ok(parse_result) => res.push(Coin {
                    denom: v.denom,
                    amount: parse_result / ONE_ETH.into(),
                }),
                Err(e) => return Err(CosmosGrpcError::ParseError { error: e }),
            }
        }
        Ok(res)
    }
}
