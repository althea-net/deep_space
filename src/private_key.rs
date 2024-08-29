use crate::mnemonic::Mnemonic;
use crate::msg::Msg;
use crate::public_key::{CosmosPublicKey, PublicKey};
use crate::utils::bytes_to_hex_str;
use crate::utils::encode_any;
use crate::utils::hex_str_to_bytes;
use crate::{coin::Fee, coin::Tip, Address};
use crate::{error::*, utils::contains_non_hex_chars};
use cosmos_sdk_proto::cosmos::crypto::secp256k1::PubKey as ProtoSecp256k1Pubkey;
use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx;
use cosmos_sdk_proto::cosmos::tx::v1beta1::{
    mode_info, AuthInfo, ModeInfo, SignDoc, SignerInfo, TxBody, TxRaw,
};
use num256::Uint256;
use prost::Message;
use secp256k1::constants::CURVE_ORDER as CurveN;
use secp256k1::Message as CurveMessage;
use secp256k1::Scalar;
use secp256k1::{All, Secp256k1};
use secp256k1::{PublicKey as PublicKeyEC, SecretKey};
use sha2::Sha512;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::str::FromStr;

thread_local! {
    pub(crate) static SECP256K1: RefCell<Secp256k1<All>> = RefCell::new(Secp256k1::new());
}

pub const DEFAULT_COSMOS_HD_PATH: &str = "m/44'/118'/0'/0/0";
pub const DEFAULT_ETHEREUM_HD_PATH: &str = "m/44'/60'/0'/0/0";

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MessageArgs {
    pub sequence: u64,
    pub fee: Fee,
    pub tip: Option<Tip>,
    pub timeout_height: u64,
    pub chain_id: String,
    pub account_number: u64,
}

struct TxParts {
    body: TxBody,
    body_buf: Vec<u8>,
    auth_info: AuthInfo,
    auth_buf: Vec<u8>,
    signatures: Vec<Vec<u8>>,
}

pub trait PrivateKey: Clone + Sized {
    fn from_secret(secret: &[u8]) -> Self
    where
        Self: Sized;

    fn from_phrase(phrase: &str, passphrase: &str) -> Result<Self, PrivateKeyError>
    where
        Self: Sized;

    fn from_hd_wallet_path(
        path: &str,
        phrase: &str,
        passphrase: &str,
    ) -> Result<Self, PrivateKeyError>
    where
        Self: Sized;

    fn to_address(&self, prefix: &str) -> Result<Address, PrivateKeyError>;

    fn get_signed_tx(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: &str,
    ) -> Result<Tx, PrivateKeyError>;

    fn sign_std_msg(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: &str,
    ) -> Result<Vec<u8>, PrivateKeyError>;
}

/// This structure represents a private key of a Cosmos Network.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct CosmosPrivateKey([u8; 32]);

impl PrivateKey for CosmosPrivateKey {
    /// Create a private key using an arbitrary slice of bytes. This function is not resistant to side
    /// channel attacks and may reveal your secret and private key. It is on the other hand more compact
    /// than the bip32+bip39 logic.
    fn from_secret(secret: &[u8]) -> CosmosPrivateKey {
        CosmosPrivateKey(from_secret(secret))
    }

    /// This function will take the key_import phrase provided by CosmosCLI
    /// and import that key. How this is done behind the scenes is quite
    /// complex. The actual seed bytes from the key_import are used to derive
    /// the root of a Bip32 HD wallet. From that root Cosmos keys are derived
    /// on the path m/44'/118'/0'/0/a where a=0 is the most common value used.
    /// Most Cosmos wallets do not even expose a=1..n much less the rest of
    /// the potential key space. This function returns m/44'/118'/0'/0/0 because
    /// that's going to be the key you want essentially all the time. If you need
    /// a different path use from_hd_wallet_path()
    fn from_phrase(phrase: &str, passphrase: &str) -> Result<CosmosPrivateKey, PrivateKeyError> {
        if phrase.is_empty() {
            return Err(HdWalletError::Bip39Error(Bip39Error::BadWordCount(0)).into());
        }
        CosmosPrivateKey::from_hd_wallet_path(DEFAULT_COSMOS_HD_PATH, phrase, passphrase)
    }

    /// Derives a private key from a mnemonic phrase and passphrase, using a BIP-44 HDPath
    /// The actual seed bytes are derived from the mnemonic phrase, which are then used to derive
    /// the root of a Bip32 HD wallet. From that application private keys are derived
    /// on the given hd_path (e.g. Cosmos' m/44'/118'/0'/0/a where a=0 is the most common value used).
    /// Most Cosmos wallets do not even expose a=1..n much less the rest of
    /// the potential key space.
    fn from_hd_wallet_path(
        hd_path: &str,
        phrase: &str,
        passphrase: &str,
    ) -> Result<CosmosPrivateKey, PrivateKeyError> {
        let secret_key = from_hd_wallet_path(hd_path, phrase, passphrase)?;
        Ok(CosmosPrivateKey(secret_key))
    }

    /// Obtain an Address for a given private key, skipping the intermediate public key
    fn to_address(&self, prefix: &str) -> Result<Address, PrivateKeyError> {
        let pubkey = self.to_public_key("")?;
        let address = pubkey.to_address_with_prefix(prefix)?;
        Ok(address)
    }

    /// Signs a transaction that contains at least one message using a single
    /// private key, returns the standard Tx type, useful for simulations
    fn get_signed_tx(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: &str,
    ) -> Result<Tx, PrivateKeyError> {
        let parts = self.build_tx(messages, args, memo)?;
        Ok(Tx {
            body: Some(parts.body),
            auth_info: Some(parts.auth_info),
            signatures: parts.signatures,
        })
    }

    /// Signs a transaction that contains at least one message using a single
    /// private key.
    fn sign_std_msg(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: &str,
    ) -> Result<Vec<u8>, PrivateKeyError> {
        let parts = self.build_tx(messages, args, memo)?;

        let tx_raw = TxRaw {
            body_bytes: parts.body_buf,
            auth_info_bytes: parts.auth_buf,
            signatures: parts.signatures,
        };

        let mut txraw_buf = Vec::new();
        tx_raw.encode(&mut txraw_buf).unwrap();
        let digest = Sha256::digest(&txraw_buf);
        trace!("TXID {}", bytes_to_hex_str(&digest));

        Ok(txraw_buf)
    }
}

impl CosmosPrivateKey {
    /// Obtain a public key for a given private key
    pub fn to_public_key(&self, prefix: &str) -> Result<CosmosPublicKey, PrivateKeyError> {
        let secp256k1 = Secp256k1::new();
        let sk = SecretKey::from_slice(&self.0)?;
        let pkey = PublicKeyEC::from_secret_key(&secp256k1, &sk);
        let compressed = pkey.serialize();
        Ok(CosmosPublicKey::from_bytes(compressed, prefix)?)
    }

    /// Internal function that that handles building a single message to sign
    /// returns an internal struct containing the parts of the built transaction
    /// in a way that's easy to mix and match for various uses and output types.
    fn build_tx(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: impl Into<String>,
    ) -> Result<TxParts, PrivateKeyError> {
        // prefix does not matter in this case, you could use a blank string
        let our_pubkey = self.to_public_key(CosmosPublicKey::DEFAULT_PREFIX)?;

        let key = ProtoSecp256k1Pubkey {
            key: our_pubkey.to_vec(),
        };

        let mut unfinished = build_unfinished_tx(
            key,
            "/cosmos.crypto.secp256k1.PubKey",
            messages,
            args.clone(),
            memo,
        );

        let sign_doc = SignDoc {
            body_bytes: unfinished.body_buf.clone(),
            auth_info_bytes: unfinished.auth_buf.clone(),
            chain_id: args.chain_id.to_string(),
            account_number: args.account_number,
        };

        // Protobuf serialization of `SignDoc`
        let mut signdoc_buf = Vec::new();
        sign_doc.encode(&mut signdoc_buf).unwrap();

        let secp256k1 = Secp256k1::new();
        let sk = SecretKey::from_slice(&self.0)?;
        let digest = Sha256::digest(&signdoc_buf);
        let msg = CurveMessage::from_digest_slice(&digest)?;
        // Sign the signdoc
        let signed = secp256k1.sign_ecdsa(&msg, &sk);
        let compact = signed.serialize_compact().to_vec();

        // Finish the TxParts and return
        unfinished.signatures = vec![compact];
        Ok(unfinished)
    }
}

impl FromStr for CosmosPrivateKey {
    type Err = PrivateKeyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match hex_str_to_bytes(s) {
            Ok(bytes) => {
                if bytes.len() == 32 {
                    let mut inner = [0; 32];
                    inner.copy_from_slice(&bytes[0..32]);
                    Ok(CosmosPrivateKey(inner))
                } else {
                    Err(PrivateKeyError::HexDecodeErrorWrongLength)
                }
            }
            Err(e) => {
                if contains_non_hex_chars(s) {
                    CosmosPrivateKey::from_phrase(s, "")
                } else {
                    Err(e.into())
                }
            }
        }
    }
}

/// This structure represents a private key of an EVM Network.
#[cfg(feature = "ethermint")]
#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub struct EthermintPrivateKey([u8; 32]);

#[cfg(feature = "ethermint")]
impl PrivateKey for EthermintPrivateKey {
    /// Create a private key using an arbitrary slice of bytes. This function is not resistant to side
    /// channel attacks and may reveal your secret and private key. It is on the other hand more compact
    /// than the bip32+bip39 logic.
    fn from_secret(secret: &[u8]) -> EthermintPrivateKey {
        EthermintPrivateKey(from_secret(secret))
    }

    /// This function will take the key_import phrase provided by CosmosCLI
    /// and import that key. How this is done behind the scenes is quite
    /// complex. The actual seed bytes from the key_import are used to derive
    /// the root of a Bip32 HD wallet. From that root Ethereum keys are derived
    /// on the path m/44'/60'/0'/0/a where a=0 is the most common value used.
    /// This function returns m/44'/60'/0'/0/0 because that's going to be the key you want
    /// essentially all the time. If you need a different path use from_hd_wallet_path()
    fn from_phrase(phrase: &str, passphrase: &str) -> Result<EthermintPrivateKey, PrivateKeyError> {
        if phrase.is_empty() {
            return Err(HdWalletError::Bip39Error(Bip39Error::BadWordCount(0)).into());
        }
        EthermintPrivateKey::from_hd_wallet_path(DEFAULT_ETHEREUM_HD_PATH, phrase, passphrase)
    }

    /// Derives a private key from a mnemonic phrase and passphrase, using a BIP-44 HDPath
    /// The actual seed bytes are derived from the mnemonic phrase, which are then used to derive
    /// the root of a Bip32 HD wallet. From that application private keys are derived
    /// on the given hd_path (e.g. Cosmos' m/44'/118'/0'/0/a where a=0 is the most common value used).
    /// Most Cosmos wallets do not even expose a=1..n much less the rest of
    /// the potential key space.
    fn from_hd_wallet_path(
        hd_path: &str,
        phrase: &str,
        passphrase: &str,
    ) -> Result<EthermintPrivateKey, PrivateKeyError> {
        let secret_key = from_hd_wallet_path(hd_path, phrase, passphrase)?;
        Ok(EthermintPrivateKey(secret_key))
    }

    fn to_address(&self, prefix: &str) -> Result<Address, PrivateKeyError> {
        let pubkey = self.to_public_key("")?;
        let address = pubkey.to_address_with_prefix(prefix)?;
        Ok(address)
    }

    fn get_signed_tx(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: &str,
    ) -> Result<Tx, PrivateKeyError> {
        let parts = self.build_tx(messages, args, memo)?;
        Ok(Tx {
            body: Some(parts.body),
            auth_info: Some(parts.auth_info),
            signatures: parts.signatures,
        })
    }

    fn sign_std_msg(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: &str,
    ) -> Result<Vec<u8>, PrivateKeyError> {
        let parts = self.build_tx(messages, args, memo)?;

        let tx_raw = TxRaw {
            body_bytes: parts.body_buf,
            auth_info_bytes: parts.auth_buf,
            signatures: parts.signatures,
        };

        let mut txraw_buf = Vec::new();
        tx_raw.encode(&mut txraw_buf).unwrap();
        let digest = Sha256::digest(&txraw_buf);
        trace!("TXID {}", bytes_to_hex_str(&digest));

        Ok(txraw_buf)
    }
}

#[cfg(feature = "ethermint")]
impl EthermintPrivateKey {
    fn to_public_key(
        self,
        prefix: &str,
    ) -> Result<crate::public_key::EthermintPublicKey, PrivateKeyError> {
        let sk = SecretKey::from_slice(&self.0)?;
        let pkey = SECP256K1.with(move |object| -> Result<_, PrivateKeyError> {
            let secp256k1 = object.borrow();
            let pkey = PublicKeyEC::from_secret_key(&secp256k1, &sk);
            // Serialize the recovered public key in uncompressed format
            Ok(pkey.serialize())
        })?;
        if pkey[1..] == [0x00u8; 64][..] {
            return Err(PrivateKeyError::ZeroPrivateKey);
        }
        let pubkey = crate::public_key::EthermintPublicKey::from_bytes(pkey, prefix)?;
        Ok(pubkey)
    }

    /// Internal function that that handles building a single message to sign
    /// returns an internal struct containing the parts of the built transaction
    /// in a way that's easy to mix and match for various uses and output types.
    fn build_tx(
        &self,
        messages: &[Msg],
        args: MessageArgs,
        memo: impl Into<String>,
    ) -> Result<TxParts, PrivateKeyError> {
        let our_pubkey = self.to_public_key(CosmosPublicKey::DEFAULT_PREFIX)?;

        // TODO: Use the ethermint proto here, not the cosmos-sdk one
        let pubkey_proto = ProtoSecp256k1Pubkey {
            key: our_pubkey.to_vec(),
        };

        let mut unfinished = build_unfinished_tx(
            pubkey_proto,
            "/ethermint.crypto.v1.ethsecp256k1.PubKey",
            messages,
            args.clone(),
            memo,
        );

        let sign_doc = SignDoc {
            body_bytes: unfinished.body_buf.clone(),
            auth_info_bytes: unfinished.auth_buf.clone(),
            chain_id: args.chain_id.to_string(),
            account_number: args.account_number,
        };

        // Protobuf serialization of `SignDoc`
        let mut signdoc_buf = Vec::new();
        sign_doc.encode(&mut signdoc_buf).unwrap();

        // Sign the signdoc
        let clarity_sk = clarity::PrivateKey::from_bytes(self.0).unwrap();

        let signed = clarity_sk.sign_insecure_msg(&signdoc_buf);

        // Finish the TxParts and return
        unfinished.signatures = vec![signed.to_bytes().to_vec()];
        Ok(unfinished)
    }
}

#[cfg(feature = "ethermint")]
impl FromStr for EthermintPrivateKey {
    type Err = PrivateKeyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match hex_str_to_bytes(s) {
            Ok(bytes) => {
                if bytes.len() == 32 {
                    let mut inner = [0; 32];
                    inner.copy_from_slice(&bytes[0..32]);
                    Ok(EthermintPrivateKey(inner))
                } else {
                    Err(PrivateKeyError::HexDecodeErrorWrongLength)
                }
            }
            Err(e) => {
                if contains_non_hex_chars(s) {
                    EthermintPrivateKey::from_phrase(s, "")
                } else {
                    Err(e.into())
                }
            }
        }
    }
}

#[cfg(feature = "ethermint")]
use clarity::PrivateKey as EthPrivateKey;
#[cfg(feature = "ethermint")]
impl From<EthPrivateKey> for EthermintPrivateKey {
    fn from(value: EthPrivateKey) -> Self {
        EthermintPrivateKey(value.to_bytes())
    }
}

/// Create a private key using an arbitrary slice of bytes. This function is not resistant to side
/// channel attacks and may reveal your secret and private key. It is on the other hand more compact
/// than the bip32+bip39 logic.
/// Note: This implementation is shared between Ethereum and standard Cosmos SDK chains
fn from_secret(secret: &[u8]) -> [u8; 32] {
    let sec_hash = Sha256::digest(secret);

    let mut i = Uint256::from_be_bytes(&sec_hash);

    // Parameters of the curve as explained in https://en.bitcoin.it/wiki/Secp256k1
    let mut n = Uint256::from_be_bytes(&CurveN);
    n -= 1u64.into();

    i %= n;
    i += 1u64.into();

    let mut result: [u8; 32] = Default::default();
    let i_bytes = i.to_be_bytes();
    result.copy_from_slice(&i_bytes);
    result
}

/// Derives a private key from a mnemonic phrase and passphrase, using a BIP-44 HDPath
/// The actual seed bytes are derived from the mnemonic phrase, which are then used to derive
/// the root of a Bip32 HD wallet. From that application private keys are derived
/// on the given hd_path (e.g. Cosmos' m/44'/118'/0'/0/a where a=0 is the most common value used).
/// Most Cosmos wallets do not even expose a=1..n much less the rest of
/// the potential key space.
/// Note: This implementation is shared between Ethereum and standard Cosmos-SDK chains
fn from_hd_wallet_path(
    hd_path: &str,
    phrase: &str,
    passphrase: &str,
) -> Result<[u8; 32], PrivateKeyError> {
    if !hd_path.starts_with('m') || hd_path.contains('\\') {
        return Err(HdWalletError::InvalidPathSpec(hd_path.to_string()).into());
    }
    let mut iterator = hd_path.split('/');
    // discard the m
    let _ = iterator.next();

    let key_import = Mnemonic::from_str(phrase)?;
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
            return Err(HdWalletError::InvalidPathSpec(hd_path.to_string()).into());
        }
    }
    Ok(secret_key)
}

/// This derives the master key from seed bytes, the actual usage is typically
/// for Cosmos key_import support, where we import a seed phrase.
fn master_key_from_seed(seed_bytes: &[u8]) -> ([u8; 32], [u8; 32]) {
    use hmac::Hmac;
    use hmac::Mac;
    type HmacSha512 = Hmac<Sha512>;

    let mut hasher = HmacSha512::new_from_slice(b"Bitcoin seed").unwrap();
    hasher.update(seed_bytes);
    let hash = hasher.finalize().into_bytes();
    let mut master_secret_key: [u8; 32] = [0; 32];
    let mut master_chain_code: [u8; 32] = [0; 32];
    master_secret_key.copy_from_slice(&hash[0..32]);
    master_chain_code.copy_from_slice(&hash[32..64]);

    // key check
    let _ = SecretKey::from_slice(&master_secret_key).unwrap();

    (master_secret_key, master_chain_code)
}

/// This keys the child key following the bip32 https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
/// specified derivation method. This method is internal because you should really be using the public API that
/// handles key path parsing.
fn get_child_key(
    k_parent: [u8; 32],
    c_parent: [u8; 32],
    i: u32,
    hardened: bool,
) -> ([u8; 32], [u8; 32]) {
    use hmac::Hmac;
    use hmac::Mac;
    type HmacSha512 = Hmac<Sha512>;

    let i = if hardened { 2u32.pow(31) + i } else { i };
    let mut hasher = HmacSha512::new_from_slice(&c_parent).unwrap();
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

    // If you wanted to do this on your own (without add_assign)
    // it would go like this
    //
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

    let k_parent = Scalar::from_be_bytes(k_parent).unwrap();

    let mut parse_i_l = SecretKey::from_slice(&l_param[0..32]).unwrap();
    parse_i_l = parse_i_l.add_tweak(&k_parent).unwrap();
    let child_key = parse_i_l;

    let mut child_key_res: [u8; 32] = [0; 32];
    child_key_res
        .copy_from_slice(&hex_str_to_bytes(&child_key.display_secret().to_string()).unwrap());
    let mut chain_code_res: [u8; 32] = [0; 32];
    chain_code_res.copy_from_slice(&l_param[32..64]);
    (child_key_res, chain_code_res)
}

fn build_unfinished_tx<P: prost::Message>(
    pubkey_proto: P,
    proto_type_url: &str,
    messages: &[Msg],
    args: MessageArgs,
    memo: impl Into<String>,
) -> TxParts {
    // Create TxBody
    let body = TxBody {
        messages: messages.iter().map(|msg| msg.0.clone()).collect(),
        memo: memo.into(),
        timeout_height: args.timeout_height,
        extension_options: Default::default(),
        non_critical_extension_options: Default::default(),
    };

    // A protobuf serialization of a TxBody
    let mut body_buf = Vec::new();
    body.encode(&mut body_buf).unwrap();

    let pk_any = encode_any(pubkey_proto, proto_type_url.to_string());

    let single = mode_info::Single { mode: 1 };

    let mode = Some(ModeInfo {
        sum: Some(mode_info::Sum::Single(single)),
    });

    let signer_info = SignerInfo {
        public_key: Some(pk_any),
        mode_info: mode,
        sequence: args.sequence,
    };

    let auth_info = AuthInfo {
        signer_infos: vec![signer_info],
        fee: Some(args.fee.into()),
        tip: args.tip.map(|v| v.into()),
    };

    // Protobuf serialization of `AuthInfo`
    let mut auth_buf = Vec::new();
    auth_info.encode(&mut auth_buf).unwrap();

    TxParts {
        body,
        body_buf,
        auth_info,
        auth_buf,
        signatures: vec![], // Unfinished
    }
}

#[test]
fn test_secret() {
    let private_key = CosmosPrivateKey::from_secret(b"mySecret");
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
        .to_public_key("cosmospub")
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
}

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

    let private_key = CosmosPrivateKey(m44h_118h_0h_0_0);
    let public_key = private_key.to_public_key("cosmospub").unwrap();
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

#[test]
fn test_cosmos_key_derivation_with_path_parsing() {
    let words = "purse sure leg gap above pull rescue glass circle attract erupt can sail gasp shy clarify inflict anger sketch hobby scare mad reject where";
    // now test with automated path parsing
    let private_key = CosmosPrivateKey::from_phrase(words, "").unwrap();
    let public_key = private_key.to_public_key("cosmospub").unwrap();
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
        let cosmos_key = CosmosPrivateKey::from_secret(&secret);
        let _cosmos_address = cosmos_key.to_public_key("cosmospub").unwrap().to_address();
    }
}

#[test]
// this tests that a bad phrase provides an error
fn test_bad_phrase() {
    let cosmos_key = CosmosPrivateKey::from_phrase("bad phrase", "");
    assert!(cosmos_key.is_err())
}

#[cfg(feature = "ethermint")]
#[test]
fn test_ethermint_compatibility() {
    // Test Evmos key:
    let expected_address = "evmos1zkunj49253lc6wgm0gp5nk8kj2naat0j8fzkfa";
    // pubkey: '{"@type":"/ethermint.crypto.v1.ethsecp256k1.PubKey","key":"Av7SwLGHN5e+WVuLgYn5rfaBdQ5WlpasMiECekGh/5P0"}'
    let mnemonic = "whisper unknown entire effort supreme believe supply position noble radar badge check cotton spider affair muffin gold bird trust venue hub core they veteran";
    let sk = EthermintPrivateKey::from_phrase(mnemonic, "").unwrap();
    let address = sk.to_address("evmos").unwrap();

    assert_eq!(expected_address, address.to_string())
}

#[cfg(feature = "ethermint")]
#[test]
fn test_ethermint_signatures() {
    use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
    use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
    use num_traits::ToPrimitive;

    // Signature of "hello world" by the below mnemonic using ethermint
    let expected_hello_sig = "1d7c2d4440e925581ee737bea00593141caeeb96925614ccfdc43ca2c9421e6676ab3fd097d366dd399110a8664fefddab9b1dc1289053f095ec285884c1bd6600";
    let mnemonic = "whisper unknown entire effort supreme believe supply position noble radar badge check cotton spider affair muffin gold bird trust venue hub core they veteran";
    let address = "evmos1zkunj49253lc6wgm0gp5nk8kj2naat0j8fzkfa".to_string();
    let sk = EthermintPrivateKey::from_phrase(mnemonic, "").unwrap();
    let msg = "hello world".to_string();
    let clarity_sk = clarity::private_key::PrivateKey::from_bytes(sk.0).unwrap();
    let signature = clarity_sk.sign_insecure_msg(msg.as_bytes());
    let mut sigbytes = signature.to_bytes();
    let v = signature.get_v();
    sigbytes[64] = (v.to_u8().unwrap()) - 27u8; // Fix some weirdness in the clarity implementation

    assert_eq!(
        sigbytes.to_vec(),
        hex_str_to_bytes(expected_hello_sig).unwrap()
    );

    /* Sample message to sign
    {
        "body": {
        "messages": [
        {
            "@type": "/cosmos.bank.v1beta1.MsgSend",
            "from_address": "evmos1zkunj49253lc6wgm0gp5nk8kj2naat0j8fzkfa",
            "to_address": "evmos1zkunj49253lc6wgm0gp5nk8kj2naat0j8fzkfa",
            "amount": [
            {
                "denom": "uatom",
                "amount": "1"
            }
            ]
        }
        ],
        "memo": "",
        "timeout_height": "0",
        "extension_options": [],
        "non_critical_extension_options": []
    },
        "auth_info": {
        "signer_infos": [
        {
            "public_key": {
            "@type": "/ethermint.crypto.v1.ethsecp256k1.PubKey",
            "key": "Av7SwLGHN5e+WVuLgYn5rfaBdQ5WlpasMiECekGh/5P0"
        },
            "mode_info": {
            "single": {
                "mode": "SIGN_MODE_DIRECT"
            }
        },
            "sequence": "0"
        }
        ],
        "fee": {
            "amount": [],
            "gas_limit": "200000",
            "payer": "",
            "granter": ""
        }
    },
        "signatures": [
        "3qEDrYCnLjIdlH8N2+8rvt9M/k8fLzWa+CdpWB9b0AsK3uZO12UAm/62uilyeiAeBroBAJ+vPDzFDjC9j963KQE="
        ]
    } */

    // The above signature entry converted to hex bytes in Go [fmt.Printf("%s", ([]byte)("3qEDrYCnLjIdlH8N2+8rvt9M/k8fLzWa+CdpWB9b0AsK3uZO12UAm/62uilyeiAeBroBAJ+vPDzFDjC9j963KQE="))]
    // let expected_msg_sig = "337145447259436e4c6a49646c48384e322b38727674394d2f6b38664c7a57612b436470574239623041734b33755a4f313255416d2f363275696c796569416542726f42414a2b7650447a46446a43396a3936334b51453d";
    let msg_send = MsgSend {
        from_address: address.clone(),
        to_address: address,
        amount: vec![Coin {
            denom: "uatom".to_string(),
            amount: "1".to_string(),
        }],
    };
    let msg_args = MessageArgs {
        sequence: 0,
        fee: Fee {
            amount: vec![],
            gas_limit: 200000,
            payer: None,
            granter: None,
        },
        tip: None,
        timeout_height: 0,
        chain_id: "chain-0".to_string(),
        account_number: 0,
    };
    let msg = Msg(encode_any(msg_send, "/cosmos.bank.v1beta1.MsgSend"));

    let _sig_tx = sk.sign_std_msg(&[msg], msg_args, "").unwrap();

    // TODO: Figure out how to verify we are signing the message correctly, this is tricky
    // println!("{:?}", sig_tx)
}

#[cfg(feature = "ethermint")]
#[test]
fn test_bank_send_msg() {
    use crate::{Coin, Contact};
    use actix_rt::System;
    use std::time::Duration;
    let runner = System::new();
    runner.block_on(async move {
        let validator_mnemonic = "story check aunt clown fence fine safe harbor transfer talent topic swing original rookie wrap movie speak message drop lava any ask soul angry";
        let user_mnemonic = "express language around erase away okay brass enough mind slogan aisle pen dignity strike roof palace inmate art sponsor exact almost cricket basket topic";
        let receiver = "gravity1secgjkfe900uef3xg3d5kvmvqrxr4yphyc2fel";

        let contact = Contact::new("http://localhost:26657", Duration::from_secs(30), "gravity").unwrap();
        let destination = Address::from_bech32(receiver.to_string()).unwrap();

        let vk = CosmosPrivateKey::from_phrase(validator_mnemonic, "").unwrap();
        let uk = EthermintPrivateKey::from_phrase(user_mnemonic, "").unwrap();

        let output = contact.send_coins(Coin { amount: 100u8.into(), denom: "ugraviton".to_string() }, None, destination, Some(Duration::from_secs(30)), vk).await;
        println!("output is {output:?}");

        let output = contact.send_coins(Coin { amount: 100u8.into(), denom: "ugraviton".to_string() }, None, destination, Some(Duration::from_secs(30)), uk).await;
        println!("output is {output:?}")
    });
}
