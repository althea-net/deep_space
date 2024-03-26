//! Contains utilities and query endpoints for use with the Cosmos bank module
//!
use super::PAGE;
use crate::error::CosmosGrpcError;
use crate::{Address, Coin, Contact};
use cosmos_sdk_proto::cosmos::bank::v1beta1::query_client::QueryClient as BankQueryClient;
use cosmos_sdk_proto::cosmos::bank::v1beta1::{
    Metadata, QueryDenomMetadataRequest, QueryDenomsMetadataRequest, QuerySupplyOfRequest,
    QueryTotalSupplyRequest,
};
use cosmos_sdk_proto::cosmos::bank::v1beta1::{QueryAllBalancesRequest, QueryBalanceRequest};
use tokio::time::timeout;

impl Contact {
    /// gets the total supply of all coins on chain
    pub async fn query_total_supply(&self) -> Result<Vec<Coin>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            BankQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.total_supply(QueryTotalSupplyRequest { pagination: PAGE }),
        )
        .await??
        .into_inner();
        let mut out = Vec::new();
        for val in res.supply {
            out.push(val.into())
        }
        Ok(out)
    }

    /// gets the supply of an individual token
    pub async fn query_supply_of(&self, denom: String) -> Result<Option<Coin>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            BankQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.supply_of(QuerySupplyOfRequest { denom }),
        )
        .await??
        .into_inner();
        match res.amount {
            Some(v) => Ok(Some(v.into())),
            None => Ok(None),
        }
    }

    /// Gets the denom metadata for every token type on the chain
    pub async fn get_all_denoms_metadata(&self) -> Result<Vec<Metadata>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            BankQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.denoms_metadata(QueryDenomsMetadataRequest { pagination: PAGE }),
        )
        .await??
        .into_inner();
        Ok(res.metadatas)
    }

    /// Gets the denom metadata for a specific token
    pub async fn get_denom_metadata(
        &self,
        denom: String,
    ) -> Result<Option<Metadata>, CosmosGrpcError> {
        let mut grpc = timeout(
            self.get_timeout(),
            BankQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            grpc.denom_metadata(QueryDenomMetadataRequest { denom }),
        )
        .await??
        .into_inner();
        Ok(res.metadata)
    }

    /// Gets the coin balances for an individual account
    pub async fn get_balances(&self, address: Address) -> Result<Vec<Coin>, CosmosGrpcError> {
        let mut bankrpc = timeout(
            self.get_timeout(),
            BankQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            bankrpc.all_balances(QueryAllBalancesRequest {
                // chain prefix is validated as part of this client, so this can't
                // panic
                address: address.to_bech32(&self.chain_prefix).unwrap(),
                pagination: PAGE,
            }),
        )
        .await??
        .into_inner();
        let balances = res.balances;
        let mut ret = Vec::new();
        for value in balances {
            ret.push(value.into());
        }
        Ok(ret)
    }

    /// Gets the balance of a single for an individual account
    pub async fn get_balance(
        &self,
        address: Address,
        denom: String,
    ) -> Result<Option<Coin>, CosmosGrpcError> {
        let mut bankrpc = timeout(
            self.get_timeout(),
            BankQueryClient::connect(self.url.clone()),
        )
        .await??;
        let res = timeout(
            self.get_timeout(),
            bankrpc.balance(QueryBalanceRequest {
                // chain prefix is validated as part of this client, so this can't
                // panic
                address: address.to_bech32(&self.chain_prefix).unwrap(),
                denom,
            }),
        )
        .await??
        .into_inner();
        match res.balance {
            Some(v) => Ok(Some(v.into())),
            None => Ok(None),
        }
    }
}
