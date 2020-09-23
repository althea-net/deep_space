///! Naive implementation of canonical JSON
use serde::Serialize;
use serde::Serializer;
use serde_json::{from_str, to_string, Error as SerdeJsonError, Value};
use std::fmt;

#[derive(Debug)]
pub enum CanonicalJsonError {
    SerializationError(SerdeJsonError),
}

impl fmt::Display for CanonicalJsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CanonicalJsonError::SerializationError(val) => write!(f, "SerializationError{}", val),
        }
    }
}

impl std::error::Error for CanonicalJsonError {}

impl From<SerdeJsonError> for CanonicalJsonError {
    fn from(error: SerdeJsonError) -> Self {
        CanonicalJsonError::SerializationError(error)
    }
}

/// Creates a canonical JSON representation of any serializable objects.
pub fn to_canonical_json(s: impl Serialize) -> Result<Vec<u8>, CanonicalJsonError> {
    // Serialize any object to String first
    let s = to_string(&s)?;
    // Deserialize into Value which would order keys
    let v: Value = from_str(&s)?;
    // Serialize that value back to string
    let s = to_string(&v)?;
    // Returns a vector of bytes
    Ok(s.as_bytes().to_vec())
}

/// Serialize a slice of bytes as a JSON object.
pub(crate) fn canonical_json_serialize<S>(x: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // NOTE: This is probably least efficient way of achieving this, and it's
    // very likely that avoiding this step and serializing a structure directly
    // could be more efficent. However, the source of this bytes is a canonical json,
    // which after deserializing into `Value` should maintain its properties, so
    // therefore this process shouldn't make any problem.
    let ss = String::from_utf8(x.to_vec()).unwrap();
    let val: Value = from_str(&ss).unwrap();
    val.serialize(s)
}

#[test]
fn test_canonical_json() {
    // A dummy serializable structure wouldn't automatically order keys
    #[derive(Serialize)]
    struct Dummy {
        b: String,
        c: String,
        a: String,
    }

    let dummy = Dummy {
        b: "B".to_string(),
        c: "C".to_string(),
        a: "A".to_string(),
    };
    let bytes = to_canonical_json(&dummy).expect("Unable to canonicalize");
    assert_eq!(bytes, b"{\"a\":\"A\",\"b\":\"B\",\"c\":\"C\"}");
}
