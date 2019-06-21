use crate::public_key::PublicKey;
use serde::{Serialize, Serializer};
use serde_json::{from_str, Value};

pub(crate) fn base64_serialize<S>(x: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&base64::encode(x))
}

pub(crate) fn string_serialize<S>(x: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ss = String::from_utf8(x.to_vec()).unwrap();
    let val: Value = from_str(&ss).unwrap();
    val.serialize(s)
}

/// Signed data
#[derive(Serialize, Debug, Default)]
pub struct Signature {
    /// Signature in a raw DER form (about 70 bytes)
    #[serde(serialize_with = "base64_serialize")]
    pub signature: Vec<u8>,
    pub pub_key: PublicKey,
    // pub account_number: String,
    // pub sequence: String,
}

#[test]
fn sig_serialize() {
    let sig = Signature {
        signature: vec![1, 2, 3, 4, 5],
        pub_key: PublicKey::default(),
    };
    let s = serde_json::to_string(&sig).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(
        v,
        json!({
            "signature": base64::encode(&[1,2,3,4,5]),
            "pub_key": {
                "type": "tendermint/PubKeySecp256k1",
                "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            },
        })
    )
}
