use crate::msg::Msg;
use crate::stdfee::StdFee;

pub struct StdSignMsg {
    chain_id: String,
    account_number: u64,
    sequence: u64,
    fee: StdFee,
    msgs: Vec<Msg>,
    memo: String,
}
