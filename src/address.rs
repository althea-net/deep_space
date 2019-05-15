use std::fmt::Write;

pub struct Address([u8; 20]);

impl Address {
    pub fn from_bytes(bytes: [u8; 20]) -> Address {
        Address(bytes)
    }
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        for &byte in self.0.iter() {
            write!(&mut s, "{:02X}", byte).expect("Unable to write");
        }
        s
    }
}
