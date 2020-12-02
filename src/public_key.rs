use crate::address::Address;
use crate::utils::hex_str_to_bytes;
use crate::utils::ByteDecodeError;
use base64::DecodeError;
use bech32::{self, FromBase32, ToBase32};
use ripemd160::Ripemd160;
use serde::{ser::SerializeMap, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::{hash::Hasher, str::FromStr};

#[derive(Debug)]
pub enum PublicKeyError {
    Bech32WrongLength,
    Bech32InvalidBase32,
    Bech32InvalidEncoding,
    HexDecodeError(ByteDecodeError),
    Base64DecodeError(DecodeError),
    HexDecodeErrorWrongLength,
    BytesDecodeErrorWrongLength,
}

impl fmt::Display for PublicKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PublicKeyError::Bech32WrongLength => write!(f, "Bech32WrongLength"),
            PublicKeyError::Bech32InvalidBase32 => write!(f, "Bech32InvalidBase32"),
            PublicKeyError::Bech32InvalidEncoding => write!(f, "Bech32InvalidEncoding"),
            PublicKeyError::HexDecodeError(val) => write!(f, "HexDecodeError {}", val),
            PublicKeyError::Base64DecodeError(val) => write!(f, "Base64DecodeError {}", val),
            PublicKeyError::BytesDecodeErrorWrongLength => {
                write!(f, "BytesDecodeError Wrong Length")
            }
            PublicKeyError::HexDecodeErrorWrongLength => write!(f, "HexDecodeError Wrong Length"),
        }
    }
}

impl std::error::Error for PublicKeyError {}

impl From<bech32::Error> for PublicKeyError {
    fn from(error: bech32::Error) -> Self {
        match error {
            bech32::Error::InvalidLength => PublicKeyError::Bech32WrongLength,
            bech32::Error::InvalidChar(_) => PublicKeyError::Bech32InvalidBase32,
            bech32::Error::InvalidData(_) => PublicKeyError::Bech32InvalidEncoding,
            bech32::Error::InvalidChecksum => PublicKeyError::Bech32InvalidEncoding,
            bech32::Error::InvalidPadding => PublicKeyError::Bech32InvalidEncoding,
            bech32::Error::MixedCase => PublicKeyError::Bech32InvalidEncoding,
            bech32::Error::MissingSeparator => PublicKeyError::Bech32InvalidEncoding,
        }
    }
}

/// Represents a public key of a given private key in the Cosmos Network.
///
/// Can be created from a private key only.
#[derive(Copy, Clone)]
pub struct PublicKey([u8; 33]);

impl Default for PublicKey {
    fn default() -> Self {
        Self([0u8; 33])
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }
}
impl Eq for PublicKey {}

impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.0.iter() {
            byte.hash(state);
        }
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO: A proper enum would be easier to serialize
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", "tendermint/PubKeySecp256k1")?;
        map.serialize_entry("value", &base64::encode(&self.0[..]))?;
        map.end()
    }
}

impl Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.iter().fmt(f)
    }
}

impl PublicKey {
    /// Create a public key using an array of bytes
    pub fn from_bytes(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }
    /// Create a public key using a slice of bytes
    pub fn from_slice(bytes: &[u8]) -> Result<Self, PublicKeyError> {
        if bytes.len() != 33 {
            return Err(PublicKeyError::BytesDecodeErrorWrongLength);
        }
        let mut result = [0u8; 33];
        result.copy_from_slice(bytes);
        Ok(Self(result))
    }

    /// Returns bytes of a given public key as a slice of bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Create an address object using a given public key.
    pub fn to_address(&self) -> Address {
        let sha256 = Sha256::digest(&self.0);
        let ripemd160 = Ripemd160::digest(&sha256);
        let mut bytes: [u8; 20] = Default::default();
        bytes.copy_from_slice(&ripemd160[..]);
        Address::from_bytes(bytes)
    }

    /// Creates amino representation of a given public key.
    ///
    /// It is used internally for bech32 encoding.
    pub fn to_amino_bytes(&self) -> Vec<u8> {
        let mut key_bytes = vec![0xEB, 0x5A, 0xE9, 0x87, 0x21];
        key_bytes.extend(self.as_bytes());
        key_bytes
    }

    /// Create a bech32 encoded public key.
    ///
    /// * `hrp` - A prefix for a bech32 encoding. By a convention
    /// Cosmos Network uses `cosmospub` as a prefix for encoding public keys.
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, PublicKeyError> {
        let bech32 = bech32::encode(&hrp.into(), self.to_amino_bytes().to_base32())?;
        Ok(bech32)
    }

    /// Parse a bech32 encoded public key
    ///
    /// * `s` - A bech32 encoded public key
    pub fn from_bech32(s: String) -> Result<PublicKey, PublicKeyError> {
        let (_hrp, data) = match bech32::decode(&s) {
            Ok(val) => val,
            Err(_e) => return Err(PublicKeyError::Bech32InvalidEncoding),
        };
        let vec: Vec<u8> = match FromBase32::from_base32(&data) {
            Ok(val) => val,
            Err(_e) => return Err(PublicKeyError::Bech32InvalidBase32),
        };
        let mut key = [0u8; 33];
        if vec.len() != 38 {
            return Err(PublicKeyError::Bech32WrongLength);
        }
        // the amnio representation prepends 5 bytes, we truncate those here
        // see to_amino_bytes()
        key.copy_from_slice(&vec[5..]);
        Ok(PublicKey(key))
    }
}

impl FromStr for PublicKey {
    type Err = PublicKeyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // interpret as bech32 if prefixed, hex otherwise
        if s.starts_with("cosmospub") {
            PublicKey::from_bech32(s.to_string())
        } else {
            if let Ok(bytes) = hex_str_to_bytes(s) {
                if bytes.len() == 33 {
                    let mut inner = [0; 33];
                    inner.copy_from_slice(&bytes[0..33]);
                    return Ok(PublicKey(inner));
                }
            }
            match base64::decode(s) {
                Ok(bytes) => {
                    if bytes.len() == 33 {
                        let mut inner = [0; 33];
                        inner.copy_from_slice(&bytes[0..33]);
                        Ok(PublicKey(inner))
                    } else {
                        Err(PublicKeyError::BytesDecodeErrorWrongLength)
                    }
                }
                Err(e) => Err(PublicKeyError::Base64DecodeError(e)),
            }
        }
    }
}

#[test]
fn check_bech32() {
    let raw_bytes = [
        0x02, 0xA1, 0x63, 0x3C, 0xAF, 0xCC, 0x01, 0xEB, 0xFB, 0x6D, 0x78, 0xE3, 0x9F, 0x68, 0x7A,
        0x1F, 0x09, 0x95, 0xC6, 0x2F, 0xC9, 0x5F, 0x51, 0xEA, 0xD1, 0x0A, 0x02, 0xEE, 0x0B, 0xE5,
        0x51, 0xB5, 0xDC,
    ];
    let public_key = PublicKey::from_slice(&raw_bytes).expect("Unable to create bytes from slice");
    assert_eq!(&public_key.0[..], &raw_bytes[..]);
    let res = public_key
        .to_bech32("cosmospub")
        .expect("Unable to convert to bech32");

    // ground truth
    assert_eq!(
        res,
        "cosmospub1addwnpepq2skx090esq7h7md0r3e76r6ruyet330e904r6k3pgpwuzl92x6actrt4uq"
    );

    // pubkey of secp256k1 private key "mySecret"
    let raw_bytes = [
        2, 150, 81, 169, 170, 196, 194, 43, 39, 179, 1, 154, 238, 109, 247, 70, 38, 110, 26, 231,
        70, 238, 121, 119, 42, 110, 94, 173, 25, 142, 189, 7, 195,
    ];
    let public_key = PublicKey::from_slice(&raw_bytes).expect("Unable to create bytes from slice");
    let res = public_key
        .to_bech32("cosmospub")
        .expect("Unable to convert to bech32");

    assert_eq!(
        res,
        "cosmospub1addwnpepq2t9r2d2cnpzkfanqxdwum0hgcnxuxh8gmh8jae2de026xvwh5ruxuv5let"
    );

    let check: Result<PublicKey, PublicKeyError> =
        "cosmospub1addwnpepq2t9r2d2cnpzkfanqxdwum0hgcnxuxh8gmh8jae2de026xvwh5ruxuv5let".parse();
    assert_eq!(check.unwrap(), public_key)
}

#[test]
fn serialize_secp256k1_pubkey() {
    let public_key = PublicKey::default();
    let serialized = serde_json::to_string(&public_key).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        deserialized,
        json!({"type": "tendermint/PubKeySecp256k1", "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"})
    );
}

#[test]
fn parse_base64_pubkey() {
    let key = "AvDDT1xY7hXKTy5ESqckNpBbQIArTkf21CfLFDnmWUY4";
    let _key: PublicKey = key.parse().unwrap();
}
