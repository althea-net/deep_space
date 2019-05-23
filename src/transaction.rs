use crate::stdtx::StdTx;

#[derive(Serialize)]
#[serde(rename = "tx")]
pub enum Transaction {
    #[serde(rename = "block")]
    Block(StdTx),
    #[serde(rename = "sync")]
    Sync(StdTx),
    #[serde(rename = "async")]
    Async(StdTx),
}
