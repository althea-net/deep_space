use serde::Serialize;
use serde::Serializer;

/// Any arbitrary message
pub struct Msg;

impl Serialize for Msg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO: It's always a dummy message
        serializer.serialize_str("TestMsg")
    }
}

#[cfg(test)]
mod tests {
    use super::Msg;
    use serde_json::{from_str, to_string, Value};
    #[test]
    fn test_serialize_msg() {
        let msg = Msg {};
        let s = to_string(&msg).expect("Unable to serialize");
        let v: Value = from_str(&s).expect("Unable to deserialize");
        assert_eq!(v, json!("TestMsg"));
    }
}
