//! Transaction messages

use bytes::BytesMut;
use prost::Message;
use prost_types::Any;

/// Transaction messages, encoded to allow arbitrary payloads
#[derive(Debug, Clone, PartialEq)]
pub struct Msg(pub(crate) Any);

impl Msg {
    /// Create a new transaction message, it's up to the user to deliver
    /// a roughly correct size amount in bytes as an argument.
    pub fn new<V: prost::Message>(type_url: impl Into<String>, value: V) -> Self {
        let size = Message::encoded_len(&value);
        let mut buf = BytesMut::with_capacity(size);
        // encoding should never fail so long as the buffer is big enough
        Message::encode(&value, &mut buf).expect("Failed to encode!");
        Msg(Any {
            type_url: type_url.into(),
            value: buf.to_vec(),
        })
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
