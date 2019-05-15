///! Private key implementation supports secp256k1
use crate::address::Address;
use crate::public_key::PublicKey;
use failure::Error;
use num_bigint::BigUint;
use num_traits::Num;
use sha2::{Digest, Sha256};

/// This structure represents a private key
#[derive(Debug, Eq, PartialEq)]
struct PrivateKey([u8; 32]);

impl PrivateKey {
    fn from_secret(secret: &[u8]) -> PrivateKey {
        let sec_hash = Sha256::digest(secret);

        let mut i = BigUint::from_str_radix(&format!("{:x}", sec_hash), 16).expect("form_radix_be");

        let mut n = BigUint::from_str_radix(
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
            16,
        )
        .expect("N");
        n -= 1u64;

        i %= n;
        i += 1u64;

        let i_bytes = i.to_bytes_be();

        let mut result: [u8; 32] = Default::default();
        result.copy_from_slice(&i.to_bytes_be());
        PrivateKey(result)
    }

    fn to_public_key(&self) -> Result<PublicKey, Error> {
        unimplemented!("to_public_key");
    }
}

#[test]
fn test_secret() {
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
    let address = public_key.address().expect("Unable to create public key");
    // assert_eq!(address, "99BCC000F7810F8BBB2AF6F03AE37D135DC87852");

    // Address =
    // let public_key = private_key.to_public_key().expect("Unable to convert to a public key");
}
