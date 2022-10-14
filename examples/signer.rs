extern crate deep_space;
use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
use deep_space::client::msgs::SECP256K1_PUBKEY_TYPE_URL;
use deep_space::Fee;
use deep_space::Msg;
use deep_space::{Coin, MessageArgs};
use deep_space::{CosmosPrivateKey, PrivateKey, PublicKey};
use std::fs::File;
use std::io::Write;

const SECRET: &str = "mySecret";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prepare keys
    println!(r#"Private key secret="{}""#, SECRET);
    let private_key = CosmosPrivateKey::from_secret(SECRET.as_bytes());
    let public_key = private_key.to_public_key("cosmospub")?;
    let address = public_key.to_address();
    // Print some diagnostics
    println!("Address: {}", address);
    println!("Public key: {}", public_key);

    let coin = Coin {
        denom: "validatortoken".to_string(),
        amount: 1u32.into(),
    };

    let send = MsgSend {
        amount: vec![coin.clone().into()],
        from_address: address.to_string(),
        to_address: "cosmos1pr2n6tfymnn2tk6rkxlu9q5q2zq5ka3wtu7sdj".to_string(),
    };

    let fee = Fee {
        amount: vec![coin],
        gas_limit: 500_000,
        granter: None,
        payer: None,
    };
    let msg = Msg::new(SECP256K1_PUBKEY_TYPE_URL, send);

    let args = MessageArgs {
        sequence: 0,
        account_number: 0,
        chain_id: "mychainid".to_string(),
        fee,
        timeout_height: 100,
    };

    let tx = private_key.sign_std_msg(&[msg], args, "")?;
    println!("TX {:?}", tx);

    let mut file = File::create("signed_msg.json")?;

    let s = serde_json::to_string_pretty(&tx)?;
    file.write_all(s.as_bytes())?;

    println!("{}", s);

    Ok(())
}
