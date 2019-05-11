///! Private key implementation supports secp256k1

struct PrivateKey([u8; 32]);

impl PrivateKey {
    fn from_secret(secret: &[u8]) -> PrivateKey {
        PrivateKey([0; 32])
    }
}

#[test]
fn test_secret() {
    let private_key = PrivateKey::from_secret(b"mySecret");
    // let public_key = private_key.to_public_key().expect("Unable to convert to a public key");
}
