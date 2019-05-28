use crate::stdtx::StdTx;

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
