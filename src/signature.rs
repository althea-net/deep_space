use crate::public_key::PublicKey;
use num256::Uint256;

/// Signed data
#[derive(Serialize, Debug, Default)]
pub struct Signature {
    pub signature: String,
    pub pub_key: PublicKey,
    pub account_number: String,
    pub sequence: Uint256,
}
