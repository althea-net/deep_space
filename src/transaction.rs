use crate::stdtx::StdTx;

/// Wraps a signed transaction together with a "mode" that denotes
/// the action that should be taken on the node after a successfuly
/// broadcasted transaction.
#[derive(Serialize, Debug)]
#[serde(tag = "mode", content = "tx")]
pub enum Transaction {
    #[serde(rename = "block")]
    Block(StdTx),
    #[serde(rename = "sync")]
    Sync(StdTx),
    #[serde(rename = "async")]
    Async(StdTx),
}
