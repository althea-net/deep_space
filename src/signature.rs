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
