use crate::public_key::PublicKey;

#[derive(Serialize, Debug)]
struct Signature {
    signature: String,
    pub_key: PublicKey,
    memo: String,
    account_number: String,
    sequence: String,
}
