use super::PAGE;
use crate::address::Address;
use crate::client::types::BaseAccount;
use crate::client::types::*;
use crate::{client::Contact, error::CosmosGrpcError};
use cosmos_sdk_proto::cosmos::auth::v1beta1::{
    query_client::QueryClient as AuthQueryClient, QueryAccountRequest, QueryAccountsRequest,
};
use tonic::Code as GrpcCode;

impl Contact {
    /// Gets account info for the provided Cosmos account using the accounts endpoint
    /// accounts do not have any info if they have no tokens or are otherwise never seen
    /// before in this case we return the special error NoToken
    pub async fn get_account_info(&self, address: Address) -> Result<BaseAccount, CosmosGrpcError> {
        match self.get_account_vesting_info(address).await {
            Ok(a) => Ok(a.get_base_account()),
            Err(e) => Err(e),
        }
    }

    /// Gets account info for the provided Cosmos account using the accounts endpoint
    /// accounts do not have any info if they have no tokens or are otherwise never seen
    /// before in this case we return the special error NoToken
    pub async fn get_account_vesting_info(
        &self,
        address: Address,
    ) -> Result<AccountType, CosmosGrpcError> {
        let mut agrpc = AuthQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let query = QueryAccountRequest {
            address: address.to_bech32(&self.chain_prefix).unwrap(),
        };
        let res = agrpc.account(query).await;
        match res {
            Ok(account) => {
                // null pointer if this fails to unwrap
                let value = account.into_inner().account.unwrap();
                AccountType::decode_from_any(value)
            }
            Err(e) => match e.code() {
                GrpcCode::NotFound => Err(CosmosGrpcError::NoToken),
                _ => Err(CosmosGrpcError::RequestError { error: e }),
            },
        }
    }

    /// Gets account info for every account on the chain, a large query
    pub async fn get_all_accounts(&self) -> Result<Vec<AccountType>, CosmosGrpcError> {
        let mut agrpc = AuthQueryClient::connect(self.url.clone())
            .await?
            .accept_gzip();
        let query = QueryAccountsRequest { pagination: PAGE };
        let res = agrpc.accounts(query).await?;
        let mut accounts = Vec::new();
        for value in res.into_inner().accounts {
            accounts.push(AccountType::decode_from_any(value)?);
        }

        Ok(accounts)
    }
}
