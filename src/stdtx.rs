use crate::signature::Signature;
use crate::stdfee::StdFee;

/// An enum that bundles the signed transaction with signatures.
#[derive(Serialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct StdTx<M> {
    pub msg: Vec<M>,
    pub fee: StdFee,
    pub memo: String,
    pub signatures: Vec<Signature>,
}
