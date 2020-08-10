///! Private key implementation supports secp256k1
#[cfg(feature = "bip39")]
use crate::bip39::Bip39Error;
#[cfg(feature = "bip39")]
use crate::bip39::Mnemonic;
use crate::public_key::PublicKey;
use crate::signature::Signature;
use crate::stdsignmsg::StdSignMsg;
use crate::stdtx::StdTx;
use crate::transaction::Transaction;
use crate::utils::hex_str_to_bytes;
use crate::utils::ByteDecodeError;
use failure::Error;
use num_bigint::BigUint;
use num_traits::Num;
use secp256k1::constants::CURVE_ORDER as CurveN;
use secp256k1::constants::GENERATOR_X as CurveG;
use secp256k1::Secp256k1;
use secp256k1::{Message, PublicKey as PublicKeyEC, SecretKey};
use sha2::Sha512;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fmt::Result as FormatResult;
use std::str::FromStr;

#[derive(Debug)]
pub enum PrivateKeyParseError {
    HexDecodeError(ByteDecodeError),
    HexDecodeErrorWrongLength,
}

impl fmt::Display for PrivateKeyParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> FormatResult {
        match self {
            PrivateKeyParseError::HexDecodeError(val) => write!(f, "PrivateKeyParseError {}", val),
            PrivateKeyParseError::HexDecodeErrorWrongLength => {
                write!(f, "PrivateKeyParseError Wrong Length")
            }
        }
    }
}

impl std::error::Error for PrivateKeyParseError {}

/// This structure represents a private key of a Cosmos Network.
#[derive(Debug, Eq, PartialEq)]
pub struct PrivateKey([u8; 32]);

impl PrivateKey {
    /// Create a private key using an arbitrary slice of bytes.
    pub fn from_secret(secret: &[u8]) -> PrivateKey {
        let sec_hash = Sha256::digest(secret);

        let mut i = BigUint::from_str_radix(&format!("{:x}", sec_hash), 16).expect("form_radix_be");

        // Parameters of the curve as explained in https://en.bitcoin.it/wiki/Secp256k1
        let mut n = BigUint::from_bytes_be(&CurveN);
        n -= 1u64;

        i %= n;
        i += 1u64;

        let mut result: [u8; 32] = Default::default();
        result.copy_from_slice(&i.to_bytes_be());
        PrivateKey(result)
    }

    #[cfg(feature = "bip39")]
    pub fn from_bip39(phrase: &str) -> Result<PrivateKey, Bip39Error> {
        let mnemonic = Mnemonic::from_str(phrase)?;
        let seed_bytes = mnemonic.to_seed("");
        Ok(PrivateKey::from_secret(&seed_bytes))
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

        // A block type is created by default
        Ok(Transaction::Block(std_tx))
    }
}

impl FromStr for PrivateKey {
    type Err = PrivateKeyParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match hex_str_to_bytes(s) {
            Ok(bytes) => {
                if bytes.len() == 32 {
                    let mut inner = [0; 32];
                    inner.copy_from_slice(&bytes[0..32]);
                    Ok(PrivateKey(inner))
                } else {
                    Err(PrivateKeyParseError::HexDecodeErrorWrongLength)
                }
            }
            Err(e) => Err(PrivateKeyParseError::HexDecodeError(e)),
        }
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
    let address = public_key.to_address();
    assert_eq!(
        address.to_string(),
        "99BCC000F7810F8BBB2AF6F03AE37D135DC87852"
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

    private_key.sign_std_msg(std_sign_msg).unwrap();
}

#[cfg(feature = "bip39")]
#[test]
fn read_private_key() {
    let words = "purse sure leg gap above pull rescue glass circle attract erupt can sail gasp shy clarify inflict anger sketch hobby scare mad reject where";
    let mnemonic = Mnemonic::from_str(words).unwrap();
    let seed_bytes = mnemonic.to_seed("");
    let (master_secret_key, master_chain_code) = master_key_from_seed(&seed_bytes);
    let (m0, _) = get_child_key(master_secret_key, master_chain_code, 0, true);
    let (m1, _) = get_child_key(master_secret_key, master_chain_code, 1, true);

    let private_key_master = PrivateKey(master_secret_key);
    let private_key_master_alt = PrivateKey::from_secret(&master_secret_key);
    let private_key_m0 = PrivateKey(m0);
    let private_key_m0_alt = PrivateKey::from_secret(&m0);
    let private_key_m1 = PrivateKey(m1);
    let private_key_m1_alt = PrivateKey::from_secret(&m1);
    println!(
        "{}\n{}\n{}\n",
        private_key_master
            .to_public_key()
            .unwrap()
            .to_bech32("cosmospub")
            .unwrap(),
        private_key_m0
            .to_public_key()
            .unwrap()
            .to_bech32("cosmospub")
            .unwrap(),
        private_key_m1
            .to_public_key()
            .unwrap()
            .to_bech32("cosmospub")
            .unwrap()
    );
    println!(
        "{}\n{}\n{}\n",
        private_key_master_alt
            .to_public_key()
            .unwrap()
            .to_bech32("cosmospub")
            .unwrap(),
        private_key_m0_alt
            .to_public_key()
            .unwrap()
            .to_bech32("cosmospub")
            .unwrap(),
        private_key_m1_alt
            .to_public_key()
            .unwrap()
            .to_bech32("cosmospub")
            .unwrap()
    );

    let public_key = private_key_master.to_public_key().unwrap();
    let address = public_key.to_address();
    assert_eq!(
        "cosmos1t0sgxmpxafdfjd3k6kgg50kdgn4muh5t0phml6",
        address.to_bech32("cosmos").unwrap()
    );
    assert_eq!(
        "cosmospub1addwnpepqfn2xmm5g2uackkn62ew309n3paf0xzhug6xshv4a4yq4algm9ksugt2dx6",
        address.to_bech32("cosmospub").unwrap()
    );
}

pub fn master_key_from_seed(seed_bytes: &[u8]) -> ([u8; 32], [u8; 32]) {
    use hmac::crypto_mac::Mac;
    use hmac::crypto_mac::NewMac;
    use hmac::Hmac;
    type HmacSha512 = Hmac<Sha512>;
    let n = BigUint::from_bytes_be(&CurveN);

    let mut hasher = HmacSha512::new_varkey(b"Bitcoin seed").unwrap();
    hasher.update(&seed_bytes);
    let hash = hasher.finalize().into_bytes();
    let mut master_secret_key: [u8; 32] = [0; 32];
    let mut master_chain_code: [u8; 32] = [0; 32];
    master_secret_key.copy_from_slice(&hash[0..32]);
    master_chain_code.copy_from_slice(&hash[32..64]);

    let key_check = BigUint::from_bytes_be(&master_secret_key);
    if key_check == 0u32.into() {
        panic!("Master key is zeros! {:?}", hash)
    } else if key_check > n {
        panic!("Master key not in curve space!")
    }

    (master_secret_key, master_chain_code)
}

/// gets the hardened child key from a parent key, chaincode, and index
pub fn get_child_key(
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
    let n = BigUint::from_bytes_be(&CurveN);

    let mut hasher = HmacSha512::new_varkey(&c_parent).unwrap();
    // https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
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

    let L = hasher.finalize().into_bytes();
    let parse_i_l = BigUint::from_bytes_be(&L[0..32]);
    let parent_key = BigUint::from_bytes_be(&k_parent);
    let child_key = (parse_i_l.clone() + parent_key) % n.clone();

    if child_key == 0u32.into() {
        panic!("child key is zeros!");
    } else if parse_i_l > n {
        panic!("child key not in curve space!")
    }

    let mut child_key_res: [u8; 32] = [0; 32];
    child_key_res.copy_from_slice(&child_key.to_bytes_be());
    let mut chain_code_res: [u8; 32] = [0; 32];
    chain_code_res.copy_from_slice(&L[32..64]);
    (child_key_res, chain_code_res)
}

#[cfg(feature = "bip39")]
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

#[cfg(feature = "bip39")]
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

    // unhardended zero
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
}
