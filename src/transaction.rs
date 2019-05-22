use crate::stdtx::StdTx;

#[derive(Serialize)]
#[serde(rename="tx")]
pub enum Transaction {
    Block(StdTx),
    Sync(StdTx),
    Async(StdTx),
}

#[test]
fn serialize_transaction() {
    let stdtx = StdTx::default();
    let tx = Transaction::Block(stdtx);
}
