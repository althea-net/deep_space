use crate::address::Address;
use crate::coin::Coin;
use serde::Serialize;

/// Any arbitrary message
#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "value")]
pub enum Msg {
    #[serde(rename = "cosmos-sdk/MsgSend")]
    SendMsg {
        from_address: Address,
        to_address: Address,
        amount: Vec<Coin>,
    },
    #[serde(rename = "deep_space/Test")]
    Test(String),
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
