extern crate deep_space;
use deep_space::client::txs_encode;
use futures::Future;

fn main() -> Result<(), Box<std::error::Error>> {
    txs_encode().wait().unwrap();
    Ok(())
}
