use crate::public_key::PublicKey;
use num256::Uint256;

#[derive(Serialize, Debug, Default)]
pub struct Signature {
    signature: String,
    pub_key: PublicKey,
    account_number: String,
    sequence: Uint256,
}
