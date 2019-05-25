extern crate deep_space;
use deep_space::client::encode;
use futures::Future;

fn main() -> Result<(), Box<std::error::Error>> {
    encode().wait().unwrap();
    Ok(())
}
