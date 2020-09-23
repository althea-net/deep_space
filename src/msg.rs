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
    #[serde(rename = "validator")]
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
    #[serde(rename = "validator")]
    pub validator: Address,
    #[serde(rename = "nonce")]
    pub nonce: Uint256,
    #[serde(rename = "signature")]
    pub eth_signature: String,
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
