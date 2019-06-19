use crate::address::Address;
use crate::canonical_json::to_canonical_json;
use crate::coin::Coin;
use crate::signature::base64_serialize;
use failure::Error;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SendMsg {
    pub from_address: Address,
    pub to_address: Address,
    pub amount: Vec<Coin>,
}

/// Any arbitrary message
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Msg {
    #[serde(rename = "cosmos-sdk/MsgSend")]
    SendMsg(SendMsg),
    #[serde(rename = "deep_space/Test")]
    Test(String),
}

impl Msg {
    pub fn to_sign_bytes(self) -> Result<Vec<u8>, Error> {
        Ok(to_canonical_json(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::Msg;
    use crate::coin::Coin;
    use serde_json::{from_str, to_string, Value};
    #[test]
    fn test_serialize_msg() {
        let msg: Msg = Msg::Test("TestMsg1".to_string());
        let s = to_string(&msg).expect("Unable to serialize");
        let v: Value = from_str(&s).expect("Unable to deserialize");
        assert_eq!(v, json!({"type": "deep_space/Test", "value": "TestMsg1"}));
    }
}
