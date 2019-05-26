extern crate deep_space;
use deep_space::client::txs_encode;
use deep_space::coin::Coin;
use deep_space::msg::Msg;
use deep_space::private_key::PrivateKey;
use deep_space::stdfee::StdFee;
use deep_space::stdsignmsg::StdSignMsg;
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

    let std_sign_msg = StdSignMsg {
        chain_id: "test-chain".to_string(),
        account_number: 1u64,
        sequence: 1u64,
        fee: StdFee {
            amount: vec![Coin {
                denom: "stake".to_string(),
                amount: 1u64.into(),
            }],
            gas: 200_000,
        },
        msgs: vec!["asdfaskdfasdfasmsg1".into()],
        memo: "hello from Curiousity".to_string(),
    };

    let tx = private_key.sign_std_msg(std_sign_msg)?;
    println!("TX {:?}", tx);
    println!("{}", serde_json::to_string_pretty(&tx)?);

    Ok(())
}
