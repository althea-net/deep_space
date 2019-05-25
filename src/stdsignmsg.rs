use crate::canonical_json::to_canonical_json;
use crate::msg::Msg;
use crate::stdfee::StdFee;
use failure::Error;
use serde_json::Value;

#[derive(Serialize, Debug, Default)]
pub struct StdSignMsg {
    pub chain_id: String,
    pub account_number: u64,
    pub sequence: u64,
    pub fee: StdFee,
    pub msgs: Vec<Msg>,
    pub memo: String,
}

impl StdSignMsg {
    /// This creates a bytes based using a canonical JSON serialization
    /// format.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(to_canonical_json(&self)?)
    }
}

#[test]
fn to_bytes() {
    let std_sign_msg = StdSignMsg::default();
    // Safe enough to compare as this is canonical JSON and the representation should be always the same
    assert_eq!(std_sign_msg.to_bytes().unwrap(), b"{\"account_number\":0,\"chain_id\":\"\",\"fee\":{\"amount\":[],\"gas\":0},\"memo\":\"\",\"msgs\":[],\"sequence\":0}".to_vec());
}
