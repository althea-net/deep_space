use crate::msg::Msg;
use crate::signature::Signature;
use crate::stdfee::StdFee;

#[derive(Serialize, Default, Debug)]
pub struct StdTx {
    pub msg: Vec<Msg>,
    pub fee: StdFee,
    pub memo: String,
    pub signature: Signature,
}
