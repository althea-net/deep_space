use crate::canonical_json::to_canonical_json;
use crate::canonical_json::CanonicalJsonError;
use crate::msg::DeepSpaceMsg;
use crate::stdfee::StdFee;
use crate::stdsigndoc::RawMessage;
use crate::stdsigndoc::StdSignDoc;
use serde::Serialize;
use std::clone::Clone;

/// This denotes a payload that should be signed.
///
/// Contains all the important data for a successful transaction, and can
/// contain other messages with instructions regarding what to do.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StdSignMsg<M> {
    /// Chain ID. Example value: "testing"
    pub chain_id: String,
    /// Account number. Example value: 1
    pub account_number: u64,
    /// Sequence number starts with 0 and should always increase
    pub sequence: u64,
    /// Fee. Amount should be `None`, and `gas` is the actual gas price.
    pub fee: StdFee,
    /// A list of messages
    pub msgs: Vec<M>,
    /// Arbitrary message that should be part of the transaction
    pub memo: String,
}

impl<M: Serialize + Clone + DeepSpaceMsg> Default for StdSignMsg<M> {
    fn default() -> StdSignMsg<M> {
        StdSignMsg {
            chain_id: "".to_string(),
            account_number: 0,
            sequence: 0,
            fee: StdFee::default(),
            msgs: Vec::new(),
            memo: "".to_string(),
        }
    }
}

impl<M: Serialize + Clone + DeepSpaceMsg> StdSignMsg<M> {
    /// This creates a bytes based message using a canonical JSON serialization
    /// format.
    pub fn to_bytes(&self) -> Result<Vec<u8>, CanonicalJsonError> {
        Ok(to_canonical_json(&self)?)
    }

    ///
    pub fn to_sign_doc(&self) -> Result<StdSignDoc, CanonicalJsonError> {
        let raw_msgs = self
            .msgs
            .clone()
            .into_iter()
            // Convert every message into a `RawMessage` which is a canonical JSON.
            .map(|msg| msg.to_sign_bytes().map(RawMessage))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(StdSignDoc {
            chain_id: self.chain_id.clone(),
            account_number: self.account_number.to_string(),
            sequence: self.sequence.to_string(),
            fee: self.fee.clone(),
            msgs: raw_msgs,
            memo: self.memo.clone(),
        })
    }
}

#[test]
fn to_bytes() {
    use crate::msg::Msg;
    let std_sign_msg: StdSignMsg<Msg> = StdSignMsg::default();
    // Safe enough to compare as this is canonical JSON and the representation should be always the same
    assert_eq!(String::from_utf8(std_sign_msg.to_bytes().unwrap()).unwrap(), "{\"account_number\":0,\"chain_id\":\"\",\"fee\":{\"amount\":[],\"gas\":\"0\"},\"memo\":\"\",\"msgs\":[],\"sequence\":0}");
}
