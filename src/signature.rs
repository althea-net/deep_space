use crate::public_key::PublicKey;
use serde::Serializer;

/// Serializes a slice of bytes in base64. For usage with serde macros.
pub(crate) fn base64_serialize<S>(x: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&base64::encode(x))
}

/// Signed data that contains both the signature, and the public key
/// used to sign it.
#[derive(Serialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Signature {
    /// Signature in a raw DER form (about 70 bytes)
    #[serde(serialize_with = "base64_serialize")]
    pub signature: Vec<u8>,
    pub pub_key: PublicKey,
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
