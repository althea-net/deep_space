//! Transaction messages

use prost_types::Any;

use crate::utils::encode_any;

/// Transaction messages, encoded to allow arbitrary payloads
#[derive(Debug, Clone, PartialEq)]
pub struct Msg(pub(crate) Any);

impl Msg {
    /// Create a new transaction message
    pub fn new<V: prost::Message>(type_url: impl Into<String>, value: V) -> Self {
        let any = encode_any(value, type_url);
        Msg(any)
    }
}

impl From<Any> for Msg {
    fn from(any: Any) -> Msg {
        Msg(any)
    }
}

impl From<Msg> for Any {
    fn from(msg: Msg) -> Any {
        msg.0
    }
}
