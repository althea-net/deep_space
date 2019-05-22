use crate::signature::Signature;
use crate::stdfee::StdFee;

#[derive(Serialize, Default)]
pub struct StdTx {
    msg: Vec<String>,
    fee: StdFee,
    memo: String,
    signature: Signature,
}
