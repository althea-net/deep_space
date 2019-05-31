extern crate deep_space;
use deep_space::address::Address;
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
    println!("Address: {}", address.to_bech32("cosmos")?);
    println!("Public key: {}", public_key.to_bech32("cosmospub")?);
    // Sign some stuff

    let std_sign_msg = StdSignMsg {
        chain_id: "testing".to_string(),
        account_number: 1u64,
        sequence: 1u64,
        fee: StdFee {
            amount: vec![Coin {
                denom: "validatortoken".to_string(),
                amount: 1u64.into(),
            }],
            gas: 200_000,
        },
        msgs: vec![Msg::SendMsg {
            from_address: address,
            to_address: Address::from_bech32(
                "osmos1zl0rh9gjf0hw9srcvhc0l4vsccqse5a6w3v66d".to_string(),
            )?,
            amount: vec![Coin {
                denom: "validatortoken".to_string(),
                amount: 1u32.into(),
            }],
        }],
        memo: "hello from Curiousity".to_string(),
    };

    let tx = private_key.sign_std_msg(std_sign_msg)?;
    println!("TX {:?}", tx);
    println!("{}", serde_json::to_string_pretty(&tx)?);

    Ok(())
}
