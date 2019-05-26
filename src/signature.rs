use crate::public_key::PublicKey;

/// Signed data
#[derive(Serialize, Debug, Default)]
pub struct Signature {
    /// Signature in a raw DER form (about 70 bytes)
    pub signature: Vec<u8>,
    pub pub_key: PublicKey,
    pub account_number: String,
    pub sequence: String,
}
