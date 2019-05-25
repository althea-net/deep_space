///! Naive implementation of canonical JSON
use serde::Serialize;
use serde_json::{from_str, to_string, Error, Value};

/// Creates a canonical JSON representation of any serializable objects.
pub fn to_canonical_json(s: impl Serialize) -> Result<Vec<u8>, Error> {
    // Serialize any object to String first
    let s = to_string(&s)?;
    // Deserialize into Value which would order keys
    let v: Value = from_str(&s)?;
    // Serialize that value back to string
    let s = to_string(&v)?;
    // Returns a vector of bytes
    Ok(s.as_bytes().to_vec())
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
