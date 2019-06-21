use crate::canonical_json::to_canonical_json;
use crate::msg::Msg;
use crate::signature::string_serialize;
use crate::stdfee::StdFee;
use failure::Error;
use serde_json::Value;

#[derive(Serialize, Debug)]
pub struct RawMessage(#[serde(serialize_with = "string_serialize")] pub Vec<u8>);

#[derive(Serialize, Debug, Default)]
pub struct StdSignDoc {
    pub chain_id: String,
    pub account_number: String,
    pub sequence: String,
    pub fee: StdFee,

    pub msgs: Vec<RawMessage>,
    pub memo: String,
}

impl StdSignDoc {
    /// This creates a bytes based using a canonical JSON serialization
    /// format.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(to_canonical_json(&self)?)
    }
}

#[test]
fn to_bytes() {
    let std_sign_msg = StdSignDoc::default();
    // Safe enough to compare as this is canonical JSON and the representation should be always the same
    assert_eq!(String::from_utf8(std_sign_msg.to_bytes().unwrap()).unwrap(), "{\"account_number\":\"\",\"chain_id\":\"\",\"fee\":{\"amount\":null,\"gas\":\"0\"},\"memo\":\"\",\"msgs\":[],\"sequence\":\"\"}");
}
