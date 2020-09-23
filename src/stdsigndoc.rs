use crate::canonical_json::{canonical_json_serialize, to_canonical_json, CanonicalJsonError};
use crate::stdfee::StdFee;

/// A raw message that gets serialized as a JSON
#[derive(Serialize, Debug)]
pub struct RawMessage(#[serde(serialize_with = "canonical_json_serialize")] pub Vec<u8>);

/// An internal structure that gets created during the signing process
/// that contains most of the properties of a `StdSignMsg` in a
/// preparation of a payload for a signing process.
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
    pub fn to_bytes(&self) -> Result<Vec<u8>, CanonicalJsonError> {
        Ok(to_canonical_json(&self)?)
    }
}

#[test]
fn to_bytes() {
    let std_sign_doc = StdSignDoc::default();
    // Safe enough to compare as this is canonical JSON and the representation should be always the same
    assert_eq!(String::from_utf8(std_sign_doc.to_bytes().unwrap()).unwrap(), "{\"account_number\":\"\",\"chain_id\":\"\",\"fee\":{\"amount\":[],\"gas\":\"0\"},\"memo\":\"\",\"msgs\":[],\"sequence\":\"\"}");
}
