use stdfee::StdFee;
use signature::Signature;

#[derive(Serialize)]
struct StdTx {
    msg: Vec<String>,
    fee: StdFee,
    signature: Signature,
}
