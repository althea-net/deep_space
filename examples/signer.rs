extern crate deep_space;
use deep_space::address::Address;
use deep_space::coin::Coin;
use deep_space::msg::Msg;
use deep_space::msg::SendMsg;
use deep_space::private_key::PrivateKey;
use deep_space::stdfee::StdFee;
use deep_space::stdsignmsg::StdSignMsg;
use deep_space::transaction::TransactionSendType;
use std::fs::File;
use std::io::Write;

const SECRET: &str = "mySecret";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prepare keys
    println!(r#"Private key secret="{}""#, SECRET);
    let private_key = PrivateKey::from_secret(SECRET.as_bytes());
    let public_key = private_key.to_public_key()?;
    let address = public_key.to_address();
    // Print some diagnostics
    println!("Address: {}", address.to_bech32("cosmos")?);
    println!("Public key: {}", public_key.to_bech32("cosmospub")?);
    // Sign some stuff

    let std_sign_msg = StdSignMsg {
        chain_id: "testing".to_string(),
        account_number: 1u64,
        sequence: 0u64,
        fee: StdFee {
            amount: vec![],
            gas: 200_000u64.into(),
        },
        msgs: vec![Msg::SendMsg(SendMsg {
            from_address: address,
            to_address: Address::from_bech32(
                "cosmos1pr2n6tfymnn2tk6rkxlu9q5q2zq5ka3wtu7sdj".to_string(),
            )?,
            amount: vec![Coin {
                denom: "validatortoken".to_string(),
                amount: 1u32.into(),
            }],
        })],
        memo: "hello from Curiosity".to_string(),
    };

    let tx = private_key.sign_std_msg(std_sign_msg, TransactionSendType::Block)?;
    println!("TX {:?}", tx);

    let mut file = File::create("signed_msg.json")?;

    let s = serde_json::to_string_pretty(&tx)?;
    file.write_all(s.as_bytes())?;

    println!("{}", s);

    Ok(())
}
