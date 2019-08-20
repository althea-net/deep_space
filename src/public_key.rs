use crate::address::Address;
use bech32::{self, ToBase32};
use failure::Error;
use ripemd160::Ripemd160;
use serde::{ser::SerializeMap, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug};

/// Represents a public key of a given private key in the Cosmos Network.
///
/// Can be created from a private key only.
pub struct PublicKey([u8; 33]);

impl Default for PublicKey {
    fn default() -> Self {
        Self([0u8; 33])
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
        self.0.into_iter().fmt(f)
    }
}

impl PublicKey {
    /// Create a public key using an array of bytes
    pub fn from_bytes(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }
    /// Create a public key using a slice of bytes
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        ensure!(bytes.len() == 33, "Invalid slice length");
        let mut result = [0u8; 33];
        result.copy_from_slice(bytes);
        Ok(Self(result))
    }

    /// Returns bytes of a given public key as a slice of bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Create an address object using a given public key.
    pub fn to_address(&self) -> Result<Address, Error> {
        let sha256 = Sha256::digest(&self.0);
        let ripemd160 = Ripemd160::digest(&sha256);
        let mut bytes: [u8; 20] = Default::default();
        bytes.copy_from_slice(&ripemd160[..]);
        Ok(Address::from_bytes(bytes))
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
    pub fn to_bech32<T: Into<String>>(&self, hrp: T) -> Result<String, Error> {
        let bech32 = bech32::encode(&hrp.into(), self.to_amino_bytes().to_base32())?;
        Ok(bech32.to_string())
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
