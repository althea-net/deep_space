///! Private key implementation supports secp256k1
use crate::public_key::PublicKey;
use crate::signature::Signature;
use crate::stdsignmsg::StdSignMsg;
use crate::stdtx::StdTx;
use crate::transaction::Transaction;
use failure::Error;
use num_bigint::BigUint;
use num_traits::Num;
use secp256k1::Secp256k1;
use secp256k1::{Message, PublicKey as PublicKeyEC, SecretKey};
use sha2::{Digest, Sha256};

/// This structure represents a private key of a Cosmos Network.
#[derive(Debug, Eq, PartialEq)]
pub struct PrivateKey([u8; 32]);

impl PrivateKey {
    /// Create a private key using an arbitrary slice of bytes.
    pub fn from_secret(secret: &[u8]) -> PrivateKey {
        let sec_hash = Sha256::digest(secret);

        let mut i = BigUint::from_str_radix(&format!("{:x}", sec_hash), 16).expect("form_radix_be");

        // Parameters of the curve as explained in https://en.bitcoin.it/wiki/Secp256k1
        let mut n = BigUint::from_str_radix(
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
            16,
        )
        .expect("N");
        n -= 1u64;

        i %= n;
        i += 1u64;

        let mut result: [u8; 32] = Default::default();
        result.copy_from_slice(&i.to_bytes_be());
        PrivateKey(result)
    }

    /// Obtain a public key for a given private key
    pub fn to_public_key(&self) -> Result<PublicKey, Error> {
        let secp256k1 = Secp256k1::new();
        let sk = SecretKey::from_slice(&self.0)?;
        let pkey = PublicKeyEC::from_secret_key(&secp256k1, &sk);
        let compressed = pkey.serialize();
        Ok(PublicKey::from_bytes(compressed))
    }

    /// Signs a transaction that contains at least one message using a single
    /// private key.
    pub fn sign_std_msg(&self, std_sign_msg: StdSignMsg) -> Result<Transaction, Error> {
        let sign_doc = std_sign_msg.clone().to_sign_doc()?;
        let bytes = sign_doc.to_bytes()?;

        // SHA256 of the sign document is signed
        let data = Sha256::digest(&bytes);

        let secp256k1 = Secp256k1::new();
        let sk = SecretKey::from_slice(&self.0)?;
        let msg = Message::from_slice(&data)?;
        // Do some signing
        let sig = secp256k1.sign(&msg, &sk);
        // Extract compact form
        let compact = sig.serialize_compact().to_vec();
        let signature = Signature {
            signature: compact.to_vec(),
            pub_key: self.to_public_key()?,
        };

        // Put a single signature in a result
        let std_tx = StdTx {
            msg: std_sign_msg.msgs,
            fee: std_sign_msg.fee,
            memo: std_sign_msg.memo,
            signatures: vec![signature],
        };

        // A block type is created by default
        Ok(Transaction::Block(std_tx))
    }
}

#[test]
fn test_secret() {
    use crate::coin::Coin;
    use crate::msg::Msg;
    use crate::stdfee::StdFee;
    let private_key = PrivateKey::from_secret(b"mySecret");
    assert_eq!(
        private_key.0,
        [
            208, 190, 115, 52, 41, 67, 47, 127, 0, 212, 37, 225, 171, 0, 52, 18, 175, 167, 93, 65,
            254, 40, 13, 139, 178, 235, 62, 130, 254, 252, 86,
            183,
            // Amino bytes: 0xe1, 0xb0, 0xf7, 0x9b, 0x20, 0xd0, 0xbe, 0x73, 0x34, 0x29, 0x43, 0x2f, 0x7f, 0x0, 0xd4, 0x25, 0xe1, 0xab, 0x0, 0x34, 0x12, 0xaf, 0xa7, 0x5d, 0x41, 0xfe, 0x28, 0xd, 0x8b, 0xb2, 0xeb, 0x3e, 0x82, 0xfe, 0xfc, 0x56, 0xb7
        ]
    );

    let public_key = private_key
        .to_public_key()
        .expect("Unable to create public key");

    assert_eq!(
        public_key.as_bytes(),
        &vec![
            2, 150, 81, 169, 170, 196, 194, 43, 39, 179, 1, 154, 238, 109, 247, 70, 38, 110, 26,
            231, 70, 238, 121, 119, 42, 110, 94, 173, 25, 142, 189, 7, 195
        ][..]
    );
    let address = public_key
        .to_address()
        .expect("Unable to create public key");
    assert_eq!(
        address.to_string(),
        "99BCC000F7810F8BBB2AF6F03AE37D135DC87852"
    );

    let std_sign_msg = StdSignMsg {
        chain_id: "test-chain".to_string(),
        account_number: 1u64,
        sequence: 1u64,
        fee: StdFee {
            amount: Some(vec![Coin {
                denom: "stake".to_string(),
                amount: 1u64.into(),
            }]),
            gas: 200_000u64.into(),
        },
        msgs: vec![Msg::Test("foo".to_string())],
        memo: "hello from Curiousity".to_string(),
    };

    private_key.sign_std_msg(std_sign_msg).unwrap();
}
