use crate::public_key::CosmosPublicKey;

/// Signed data that contains both the signature, and the public key
/// used to sign it.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Signature {
    pub signature: Vec<u8>,
    pub pub_key: CosmosPublicKey, // TODO: fix this, should be any private key or we need two sig types
}
