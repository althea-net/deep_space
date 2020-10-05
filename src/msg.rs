use crate::address::Address;
use crate::canonical_json::to_canonical_json;
use crate::canonical_json::CanonicalJsonError;
use crate::coin::Coin;

/// This trait allows anyone to implement their own Msg enum. This is useful
/// for various modules that may have their own custom message types. Keep in
/// mind you need to use the same serde tags as the Msg type itself including
/// a rename indicating your module name and message name. View the source
/// of the Msg struct for an example of this.
pub trait DeepSpaceMsg {
    fn to_sign_bytes(&self) -> Result<Vec<u8>, CanonicalJsonError>;
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct SendMsg {
    pub from_address: Address,
    pub to_address: Address,
    pub amount: Vec<Coin>,
}

/// Native Cosmos messages, such as transactions, staking etc
/// Currently only MsgSend is implemented. To provide module
/// specific messages implement your own version of this enum
/// and the trait DeepSpaceMsg. You will also need to duplicate
/// the serde tags.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum Msg {
    #[serde(rename = "cosmos-sdk/MsgSend")]
    SendMsg(SendMsg),

    #[serde(rename = "deep_space/Test")]
    Test(String),
}

impl DeepSpaceMsg for Msg {
    fn to_sign_bytes(&self) -> Result<Vec<u8>, CanonicalJsonError> {
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
