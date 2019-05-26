extern crate deep_space;
use deep_space::client::txs_encode;
use deep_space::private_key::PrivateKey;
use futures::Future;

const SECRET: &'static str = "mySecret";

fn main() -> Result<(), Box<std::error::Error>> {
    // Prepare keys
    println!(r#"Private key secret="{}""#, SECRET);
    let private_key = PrivateKey::from_secret(SECRET.as_bytes());
    let public_key = private_key.to_public_key()?;
    let address = public_key.to_address()?;
    // Print some diagnostics
    println!("Address: {}", address.to_string());
    println!("Bech32: {}", public_key.to_bech32("cosmospub")?);
    // Sign some stuff
    Ok(())
}
