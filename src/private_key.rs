///! Private key implementation supports secp256k1
#[cfg(feature = "key_import")]
use crate::mnemonic::Bip39Error;
#[cfg(feature = "key_import")]
use crate::mnemonic::Mnemonic;
use crate::public_key::PublicKey;
use crate::signature::Signature;
use crate::stdsignmsg::StdSignMsg;
use crate::stdtx::StdTx;
use crate::transaction::Transaction;
use crate::transaction::TransactionSendType;
use crate::utils::hex_str_to_bytes;
use crate::utils::ByteDecodeError;
use crate::{canonical_json::CanonicalJsonError, msg::DeepSpaceMsg};
use num_bigint::BigUint;
use secp256k1::constants::CURVE_ORDER as CurveN;
use secp256k1::Error as CurveError;
use secp256k1::Secp256k1;
use secp256k1::{Message, PublicKey as PublicKeyEC, SecretKey};
#[cfg(feature = "key_import")]
use sha2::Sha512;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fmt::Result as FormatResult;
use std::str::FromStr;

#[derive(Debug)]
pub enum PrivateKeyError {
    HexDecodeError(ByteDecodeError),
    HexDecodeErrorWrongLength,
    CurveError(CurveError),
    CanonicalJsonError(CanonicalJsonError),
}

impl fmt::Display for PrivateKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> FormatResult {
        match self {
            PrivateKeyError::HexDecodeError(val) => write!(f, "PrivateKeyError {}", val),
            PrivateKeyError::HexDecodeErrorWrongLength => write!(f, "PrivateKeyError Wrong Length"),
            PrivateKeyError::CurveError(val) => write!(f, "Secp256k1 Error {}", val),
            PrivateKeyError::CanonicalJsonError(val) => write!(f, "CanonicalJsonError {}", val),
        }
    }
}

impl std::error::Error for PrivateKeyError {}

impl From<CurveError> for PrivateKeyError {
    fn from(error: CurveError) -> Self {
        PrivateKeyError::CurveError(error)
    }
}

impl From<CanonicalJsonError> for PrivateKeyError {
    fn from(error: CanonicalJsonError) -> Self {
        PrivateKeyError::CanonicalJsonError(error)
    }
}

#[cfg(feature = "key_import")]
#[derive(Debug)]
pub enum HDWalletError {
    Bip39Error(Bip39Error),
    InvalidPathSpec(String),
}

#[cfg(feature = "key_import")]
impl fmt::Display for HDWalletError {
    fn fmt(&self, f: &mut fmt::Formatter) -> FormatResult {
        match self {
            HDWalletError::Bip39Error(val) => write!(f, "{}", val),
            HDWalletError::InvalidPathSpec(val) => write!(f, "HDWalletError invalid path {}", val),
        }
    }
}

#[cfg(feature = "key_import")]
impl std::error::Error for HDWalletError {}

/// This structure represents a private key of a Cosmos Network.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct PrivateKey([u8; 32]);

impl PrivateKey {
    /// Create a private key using an arbitrary slice of bytes. This function is not resistant to side
    /// channel attacks and may reveal your secret and private key. It is on the other hand more compact
    /// than the bip32+bip39 logic.
    pub fn from_secret(secret: &[u8]) -> PrivateKey {
        let sec_hash = Sha256::digest(secret);

        let mut i = BigUint::from_bytes_be(&sec_hash);

        // Parameters of the curve as explained in https://en.bitcoin.it/wiki/Secp256k1
        let mut n = BigUint::from_bytes_be(&CurveN);
        n -= 1u64;

        i %= n;
        i += 1u64;

        let mut result: [u8; 32] = Default::default();
        let mut i_bytes = i.to_bytes_be();
        // key has leading or trailing zero that's not displayed
        // by default since this is a big int library missing a defined
        // integer width.
        while i_bytes.len() < 32 {
            i_bytes.push(0);
        }
        result.copy_from_slice(&i_bytes);
        PrivateKey(result)
    }

    #[cfg(feature = "key_import")]
    /// This function will take the key_import phrase provided by CosmosCLI
    /// and import that key. How this is done behind the scenes is quite
    /// complex. The actual seed bytes from the key_import are used to derive
    /// the root of a Bip32 HD wallet. From that root Cosmos keys are derived
    /// on the path m/44'/118'/0'/0/a where a=0 is the most common value used.
    /// Most Cosmos wallets do not even expose a=1..n much less the rest of
    /// the potential key space. This function returns m/44'/118'/0'/0/0 because
    /// that's going to be the key you want essentially all the time. If you need
    /// a different path use from_hd_wallet_path()
    pub fn from_phrase(phrase: &str, passphrase: &str) -> Result<PrivateKey, HDWalletError> {
        if phrase.is_empty() {
            return Err(HDWalletError::Bip39Error(Bip39Error::BadWordCount(0)));
        }
        PrivateKey::from_hd_wallet_path("m/44'/118'/0'/0/0", phrase, passphrase)
    }

    #[cfg(feature = "key_import")]
    pub fn from_hd_wallet_path(
        path: &str,
        phrase: &str,
        passphrase: &str,
    ) -> Result<PrivateKey, HDWalletError> {
        if !path.starts_with('m') || path.contains('\\') {
            return Err(HDWalletError::InvalidPathSpec(path.to_string()));
        }
        let mut iterator = path.split('/');
        // discard the m
        let _ = iterator.next();

        let key_import = Mnemonic::from_str(phrase).unwrap();
        let seed_bytes = key_import.to_seed(passphrase);
        let (master_secret_key, master_chain_code) = master_key_from_seed(&seed_bytes);
        let mut secret_key = master_secret_key;
        let mut chain_code = master_chain_code;

        for mut val in iterator {
            let mut hardened = false;
            if val.contains('\'') {
                hardened = true;
                val = val.trim_matches('\'');
            }
            if let Ok(parsed_int) = val.parse() {
                let (s, c) = get_child_key(secret_key, chain_code, parsed_int, hardened);
                secret_key = s;
                chain_code = c;
            } else {
                return Err(HDWalletError::InvalidPathSpec(path.to_string()));
            }
        }
        Ok(PrivateKey(secret_key))
    }

    /// Obtain a public key for a given private key
    pub fn to_public_key(&self) -> Result<PublicKey, PrivateKeyError> {
        let secp256k1 = Secp256k1::new();
        let sk = SecretKey::from_slice(&self.0)?;
        let pkey = PublicKeyEC::from_secret_key(&secp256k1, &sk);
        let compressed = pkey.serialize();
        Ok(PublicKey::from_bytes(compressed))
    }

    /// Signs a transaction that contains at least one message using a single
    /// private key.
    pub fn sign_std_msg<M: serde::Serialize + std::clone::Clone + DeepSpaceMsg>(
        &self,
        std_sign_msg: StdSignMsg<M>,
        mode: TransactionSendType,
    ) -> Result<Transaction<M>, PrivateKeyError> {
        let sign_doc = std_sign_msg.to_sign_doc()?;
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

        Ok(match mode {
            TransactionSendType::Async => Transaction::Async(std_tx),
            TransactionSendType::Block => Transaction::Block(std_tx),
            TransactionSendType::Sync => Transaction::Sync(std_tx),
        })
    }
}

impl FromStr for PrivateKey {
    type Err = PrivateKeyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match hex_str_to_bytes(s) {
            Ok(bytes) => {
                if bytes.len() == 32 {
                    let mut inner = [0; 32];
                    inner.copy_from_slice(&bytes[0..32]);
                    Ok(PrivateKey(inner))
                } else {
                    Err(PrivateKeyError::HexDecodeErrorWrongLength)
                }
            }
            Err(e) => Err(PrivateKeyError::HexDecodeError(e)),
        }
    }
}

#[cfg(feature = "key_import")]
/// This derives the master key from seed bytes, the actual usage is typically
/// for Cosmos key_import support, where we import a seed phrase.
fn master_key_from_seed(seed_bytes: &[u8]) -> ([u8; 32], [u8; 32]) {
    use hmac::crypto_mac::Mac;
    use hmac::crypto_mac::NewMac;
    use hmac::Hmac;
    type HmacSha512 = Hmac<Sha512>;

    let mut hasher = HmacSha512::new_varkey(b"Bitcoin seed").unwrap();
    hasher.update(&seed_bytes);
    let hash = hasher.finalize().into_bytes();
    let mut master_secret_key: [u8; 32] = [0; 32];
    let mut master_chain_code: [u8; 32] = [0; 32];
    master_secret_key.copy_from_slice(&hash[0..32]);
    master_chain_code.copy_from_slice(&hash[32..64]);

    // key check
    let _ = SecretKey::from_slice(&master_secret_key).unwrap();

    (master_secret_key, master_chain_code)
}

#[cfg(feature = "key_import")]
/// This keys the child key following the bip32 https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
/// specified derivation method. This method is internal because you should really be using the public API that
/// handles key path parsing.
fn get_child_key(
    k_parent: [u8; 32],
    c_parent: [u8; 32],
    i: u32,
    hardened: bool,
) -> ([u8; 32], [u8; 32]) {
    use hmac::crypto_mac::Mac;
    use hmac::crypto_mac::NewMac;
    use hmac::Hmac;
    type HmacSha512 = Hmac<Sha512>;

    let i = if hardened { 2u32.pow(31) + i } else { i };
    let mut hasher = HmacSha512::new_varkey(&c_parent).unwrap();
    if hardened {
        hasher.update(&[0u8]);
        hasher.update(&k_parent);
    } else {
        let scep = Secp256k1::new();
        let private_key = SecretKey::from_slice(&k_parent).unwrap();
        let public_key = PublicKeyEC::from_secret_key(&scep, &private_key);
        hasher.update(&public_key.serialize());
    }
    hasher.update(&i.to_be_bytes());

    let l_param = hasher.finalize().into_bytes();

    // If you wanted to do this on your own it would go like this
    // but this implementation is not constant time and performs
    // allocations, opening us up to side channel attacks.
    // that being said our Hmac and SHA libraries don't clearly
    // indicate if they are constant time. So this ship may have
    // already sailed
    //
    // let n = BigUint::from_bytes_be(&CurveN);
    // let parse_i_l = BigUint::from_bytes_be(&l_param[0..32]);
    // let parent_key = BigUint::from_bytes_be(&k_parent);
    // let child_key = (parse_i_l.clone() + parent_key) % n.clone();
    // if child_key == 0u32.into() {
    //     panic!("child key is zeros!");
    // } else if parse_i_l > n {
    //     panic!("child key not in curve space!")
    // }

    let mut parse_i_l = SecretKey::from_slice(&l_param[0..32]).unwrap();
    parse_i_l.add_assign(&k_parent).unwrap();
    let child_key = parse_i_l;

    let mut child_key_res: [u8; 32] = [0; 32];
    child_key_res.copy_from_slice(&hex_str_to_bytes(&format!("{:x}", child_key)).unwrap());
    let mut chain_code_res: [u8; 32] = [0; 32];
    chain_code_res.copy_from_slice(&l_param[32..64]);
    (child_key_res, chain_code_res)
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
    let address = public_key.to_address();
    assert_eq!(
        address.to_string(),
        "cosmos1nx7vqq8hsy8chwe27mcr4cmazdwus7zjl2ds0p"
    );

    let std_sign_msg = StdSignMsg {
        chain_id: "test-chain".to_string(),
        account_number: 1u64,
        sequence: 1u64,
        fee: StdFee {
            amount: vec![Coin {
                denom: "stake".to_string(),
                amount: 1u64.into(),
            }],
            gas: 200_000u64.into(),
        },
        msgs: vec![Msg::Test("foo".to_string())],
        memo: "hello from Curiousity".to_string(),
    };

    private_key
        .sign_std_msg(std_sign_msg, TransactionSendType::Block)
        .unwrap();
}

#[cfg(feature = "key_import")]
#[test]
fn test_cosmos_key_derivation_manual() {
    let words = "purse sure leg gap above pull rescue glass circle attract erupt can sail gasp shy clarify inflict anger sketch hobby scare mad reject where";

    // first test the underlying functions manually
    let key_import = Mnemonic::from_str(words).unwrap();
    let seed_bytes = key_import.to_seed("");
    let (master_secret_key, master_chain_code) = master_key_from_seed(&seed_bytes);
    let (m44h, c44h) = get_child_key(master_secret_key, master_chain_code, 44, true);
    let (m44h_118h, c44h_118h) = get_child_key(m44h, c44h, 118, true);
    let (m44h_118h_0h, c44h_118h_0h) = get_child_key(m44h_118h, c44h_118h, 0, true);
    let (m44h_118h_0h_0, c44h_118h_0h_0) = get_child_key(m44h_118h_0h, c44h_118h_0h, 0, false);
    let (m44h_118h_0h_0_0, _c44h_118h_0h_0_0) =
        get_child_key(m44h_118h_0h_0, c44h_118h_0h_0, 0, false);

    let private_key = PrivateKey(m44h_118h_0h_0_0);
    let public_key = private_key.to_public_key().unwrap();
    let address = public_key.to_address();
    assert_eq!(
        address.to_bech32("cosmos").unwrap(),
        "cosmos1t0sgxmpxafdfjd3k6kgg50kdgn4muh5t0phml6",
    );
    assert_eq!(
        public_key.to_bech32("cosmospub").unwrap(),
        "cosmospub1addwnpepqfn2xmm5g2uackkn62ew309n3paf0xzhug6xshv4a4yq4algm9ksugt2dx6",
    );
}

#[cfg(feature = "key_import")]
#[test]
fn test_cosmos_key_derivation_with_path_parsing() {
    let words = "purse sure leg gap above pull rescue glass circle attract erupt can sail gasp shy clarify inflict anger sketch hobby scare mad reject where";
    // now test with automated path parsing
    let private_key = PrivateKey::from_phrase(words, "").unwrap();
    let public_key = private_key.to_public_key().unwrap();
    let address = public_key.to_address();
    assert_eq!(
        address.to_bech32("cosmos").unwrap(),
        "cosmos1t0sgxmpxafdfjd3k6kgg50kdgn4muh5t0phml6",
    );
    assert_eq!(
        public_key.to_bech32("cosmospub").unwrap(),
        "cosmospub1addwnpepqfn2xmm5g2uackkn62ew309n3paf0xzhug6xshv4a4yq4algm9ksugt2dx6",
    );
}

#[cfg(feature = "key_import")]
#[test]
/// This tests deriving HD wallet keys from a given seed and i value
fn test_vector_hardened() {
    let seed = hex_str_to_bytes("000102030405060708090a0b0c0d0e0f").unwrap();
    let (master_privkey, master_chaincode) = master_key_from_seed(&seed);
    let correct_master_privkey =
        hex_str_to_bytes("e8f32e723decf4051aefac8e2c93c9c5b214313817cdb01a1494b917c8436b35")
            .unwrap();
    let correct_master_chaincode =
        hex_str_to_bytes("873dff81c02f525623fd1fe5167eac3a55a049de3d314bb42ee227ffed37d508")
            .unwrap();

    assert_eq!(master_privkey.len(), correct_master_privkey.len());
    assert_eq!(master_chaincode.len(), correct_master_chaincode.len());
    assert_eq!(master_privkey.to_vec(), correct_master_privkey);
    assert_eq!(master_chaincode.to_vec(), correct_master_chaincode);

    // now we try deriving some child keys

    // hardened zero
    let (m0_dash, c0_dash) = get_child_key(master_privkey, master_chaincode, 0, true);
    let correct_m0_dash_chaincode =
        hex_str_to_bytes("47fdacbd0f1097043b78c63c20c34ef4ed9a111d980047ad16282c7ae6236141")
            .unwrap();
    let correct_m0_dash_privkey =
        hex_str_to_bytes("edb2e14f9ee77d26dd93b4ecede8d16ed408ce149b6cd80b0715a2d911a0afea")
            .unwrap();
    assert_eq!(m0_dash.len(), correct_m0_dash_privkey.len());
    assert_eq!(c0_dash.len(), correct_m0_dash_chaincode.len());
    assert_eq!(m0_dash.to_vec(), correct_m0_dash_privkey);
    assert_eq!(c0_dash.to_vec(), correct_m0_dash_chaincode);
}

#[cfg(feature = "key_import")]
#[test]
/// This tests deriving HD wallet keys from a given seed and i value
fn test_vector_unhardened() {
    // new seed for unhardened test
    let seed = hex_str_to_bytes("fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a29f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542").unwrap();
    let (master_privkey, master_chaincode) = master_key_from_seed(&seed);
    let correct_master_privkey =
        hex_str_to_bytes("4b03d6fc340455b363f51020ad3ecca4f0850280cf436c70c727923f6db46c3e")
            .unwrap();
    let correct_master_chaincode =
        hex_str_to_bytes("60499f801b896d83179a4374aeb7822aaeaceaa0db1f85ee3e904c4defbd9689")
            .unwrap();

    assert_eq!(master_privkey.len(), correct_master_privkey.len());
    assert_eq!(master_chaincode.len(), correct_master_chaincode.len());
    assert_eq!(master_privkey.to_vec(), correct_master_privkey);
    assert_eq!(master_chaincode.to_vec(), correct_master_chaincode);

    //m/0
    let (m0, c0) = get_child_key(master_privkey, master_chaincode, 0, false);
    let correct_m0_chaincode =
        hex_str_to_bytes("f0909affaa7ee7abe5dd4e100598d4dc53cd709d5a5c2cac40e7412f232f7c9c")
            .unwrap();
    let correct_m0_privkey =
        hex_str_to_bytes("abe74a98f6c7eabee0428f53798f0ab8aa1bd37873999041703c742f15ac7e1e")
            .unwrap();
    assert_eq!(m0.len(), correct_m0_privkey.len());
    assert_eq!(c0.len(), correct_m0_chaincode.len());
    assert_eq!(m0.to_vec(), correct_m0_privkey);
    assert_eq!(c0.to_vec(), correct_m0_chaincode);

    //m/0/2147483647'
    let (m0, c0) = get_child_key(m0, c0, 2147483647, true);
    let correct_m0_chaincode =
        hex_str_to_bytes("be17a268474a6bb9c61e1d720cf6215e2a88c5406c4aee7b38547f585c9a37d9")
            .unwrap();
    let correct_m0_privkey =
        hex_str_to_bytes("877c779ad9687164e9c2f4f0f4ff0340814392330693ce95a58fe18fd52e6e93")
            .unwrap();
    assert_eq!(m0.len(), correct_m0_privkey.len());
    assert_eq!(c0.len(), correct_m0_chaincode.len());
    assert_eq!(m0.to_vec(), correct_m0_privkey);
    assert_eq!(c0.to_vec(), correct_m0_chaincode);

    //m/0/2147483647'/1
    let (m0, c0) = get_child_key(m0, c0, 1, false);
    let correct_m0_chaincode =
        hex_str_to_bytes("f366f48f1ea9f2d1d3fe958c95ca84ea18e4c4ddb9366c336c927eb246fb38cb")
            .unwrap();
    let correct_m0_privkey =
        hex_str_to_bytes("704addf544a06e5ee4bea37098463c23613da32020d604506da8c0518e1da4b7")
            .unwrap();
    assert_eq!(m0.len(), correct_m0_privkey.len());
    assert_eq!(c0.len(), correct_m0_chaincode.len());
    assert_eq!(m0.to_vec(), correct_m0_privkey);
    assert_eq!(c0.to_vec(), correct_m0_chaincode);

    //m/0/2147483647'/1/2147483646'
    let (m0, c0) = get_child_key(m0, c0, 2147483646, true);
    let correct_m0_chaincode =
        hex_str_to_bytes("637807030d55d01f9a0cb3a7839515d796bd07706386a6eddf06cc29a65a0e29")
            .unwrap();
    let correct_m0_privkey =
        hex_str_to_bytes("f1c7c871a54a804afe328b4c83a1c33b8e5ff48f5087273f04efa83b247d6a2d")
            .unwrap();
    assert_eq!(m0.len(), correct_m0_privkey.len());
    assert_eq!(c0.len(), correct_m0_chaincode.len());
    assert_eq!(m0.to_vec(), correct_m0_privkey);
    assert_eq!(c0.to_vec(), correct_m0_chaincode);
}

#[test]
// this tests generating many thousands of private keys
fn test_many_key_generation() {
    use rand::Rng;
    for _ in 0..1000 {
        let mut rng = rand::thread_rng();
        let secret: [u8; 32] = rng.gen();
        let cosmos_key = PrivateKey::from_secret(&secret);
        let _cosmos_address = cosmos_key.to_public_key().unwrap().to_address();
    }
}
