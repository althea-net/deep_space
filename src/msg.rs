use crate::address::Address;
use crate::canonical_json::to_canonical_json;
use crate::canonical_json::CanonicalJsonError;
use crate::coin::Coin;
#[cfg(feature = "peggy")]
use clarity::Address as EthAddress;
#[cfg(feature = "peggy")]
use num256::Uint256;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct SendMsg {
    pub from_address: Address,
    pub to_address: Address,
    pub amount: Vec<Coin>,
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct SetEthAddressMsg {
    #[serde(rename = "address")]
    pub eth_address: EthAddress,
    pub validator: Address,
    /// a hex encoded string representing the Ethereum signature
    #[serde(rename = "signature")]
    pub eth_signature: String,
}
#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct ValsetRequestMsg {
    pub requester: Address,
}
/// a transaction we send to submit a valset confirmation signature
#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct ValsetConfirmMsg {
    pub validator: Address,
    pub nonce: Uint256,
    #[serde(rename = "signature")]
    pub eth_signature: String,
}

/// a transaction we send to move funds from Cosmos to Ethereum
#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct SendToEthMsg {
    pub sender: Address,
    pub dest_address: EthAddress,
    pub send: Coin,
    pub bridge_fee: Coin,
}

/// This message requests that a batch be created on the Cosmos chain, this
/// may or may not actually trigger a batch to be created depending on the
/// internal batch creation rules. Said batch will be of arbitrary size also
/// depending on those rules. What this message does determine is the coin
/// type of the batch. Since all batches only move a single asset within them.
#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct RequestBatchMsg {
    pub requester: Address,
    pub denom: String,
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct ConfirmBatchMsg {
    pub nonce: Uint256,
    pub validator: Address,
    /// a hex encoded string representing the Ethereum signature
    #[serde(rename = "signature")]
    pub eth_signature: String,
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct ERC20Token {
    amount: Uint256,
    symbol: String,
    #[serde(rename = "token_contract_address")]
    token_contract_address: EthAddress,
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct EthereumBridgeDepositClaim {
    pub nonce: Uint256,
    pub validator: Address,
    /// a hex encoded string representing the Ethereum signature
    #[serde(rename = "signature")]
    pub eth_signature: String,
    #[serde(rename = "ERC20Token")]
    pub erc20_token: ERC20Token,
    pub ethereum_sender: EthAddress,
    pub cosmos_receiver: Address,
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct EthereumBridgeWithdrawBatchClaim {
    pub nonce: Uint256,
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct EthereumBridgeMultiSigUpdateClaim {
    pub nonce: Uint256,
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub enum EthereumBridgeClaim {
    EthereumBridgeDepositClaim(EthereumBridgeDepositClaim),
    EthereumBridgeMultiSigUpdateClaim(EthereumBridgeMultiSigUpdateClaim),
    EthereumBridgeWithdrawBatchClaim(EthereumBridgeWithdrawBatchClaim),
}

#[cfg(feature = "peggy")]
#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct CreateEthereumClaimsMsg {
    pub ethereum_chain_id: Uint256,
    pub bridge_contract_address: EthAddress,
    pub validator: Address,
    pub claims: Vec<EthereumBridgeClaim>,
}

/// Any arbitrary message
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(tag = "type", content = "value")]
pub enum Msg {
    #[serde(rename = "cosmos-sdk/MsgSend")]
    SendMsg(SendMsg),

    #[cfg(feature = "peggy")]
    #[serde(rename = "peggy/MsgSetEthAddress")]
    SetEthAddressMsg(SetEthAddressMsg),

    #[cfg(feature = "peggy")]
    #[serde(rename = "peggy/MsgValsetRequest")]
    ValsetRequestMsg(ValsetRequestMsg),

    #[cfg(feature = "peggy")]
    #[serde(rename = "peggy/MsgValsetConfirm")]
    ValsetConfirmMsg(ValsetConfirmMsg),

    #[cfg(feature = "peggy")]
    #[serde(rename = "peggy/MsgSendToEth")]
    SendToEthMsg(SendToEthMsg),

    #[cfg(feature = "peggy")]
    #[serde(rename = "peggy/MsgRequestBatch")]
    RequestBatchMsg(RequestBatchMsg),

    #[cfg(feature = "peggy")]
    #[serde(rename = "peggy/MsgConfirmBatch")]
    ConfirmBatchMsg(ConfirmBatchMsg),

    #[cfg(feature = "peggy")]
    #[serde(rename = "peggy/MsgCreateEthereumClaims")]
    CreateEthereumClaimsMsg(CreateEthereumClaimsMsg),

    #[serde(rename = "deep_space/Test")]
    Test(String),
}

impl Msg {
    pub fn to_sign_bytes(&self) -> Result<Vec<u8>, CanonicalJsonError> {
        Ok(to_canonical_json(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::Msg;
    use serde_json::{from_str, to_string, Value};
    #[test]
    fn test_serialize_msg() {
        let msg: Msg = Msg::Test("TestMsg1".to_string());
        let s = to_string(&msg).expect("Unable to serialize");
        let v: Value = from_str(&s).expect("Unable to deserialize");
        assert_eq!(v, json!({"type": "deep_space/Test", "value": "TestMsg1"}));
    }
}
