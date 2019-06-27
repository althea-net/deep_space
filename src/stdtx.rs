use crate::msg::Msg;
use crate::signature::Signature;
use crate::stdfee::StdFee;

/// An enum that bundles the signed transaction with signatures.
#[derive(Serialize, Default, Debug)]
pub struct StdTx {
    pub msg: Vec<Msg>,
    pub fee: StdFee,
    pub memo: String,
    pub signatures: Vec<Signature>,
}
