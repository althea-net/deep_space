use crate::stdtx::StdTx;

/// Wraps a signed transaction together with a "mode" that denotes
/// the action that should be taken on the node after a successfuly
/// broadcasted transaction.
#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "mode", content = "tx")]
pub enum Transaction<M> {
    #[serde(rename = "block")]
    Block(StdTx<M>),
    #[serde(rename = "sync")]
    Sync(StdTx<M>),
    #[serde(rename = "async")]
    Async(StdTx<M>),
}

pub enum TransactionSendType {
    /// literally blocks until the transaction is in the blockchain, very useful
    /// if you are willing to have a long timeout and want to be sure that your
    /// transaction gets in right then and there. Be cautious using this in high
    /// reliability use cases.
    Block,
    /// Sync means that the full node will take some time to validate your transaction
    /// and provide you a log with any errors it encounters immediately. A txhash is also
    /// returned. This will not ensure your transaction actually gets in or even error out
    /// if your tx has any non-obvious problems. Like too little gas.
    Sync,
    /// Returns immediately providing a txhash. This is the minimum amount of validation
    /// you can get away with and still have your transaction handed over to a full node
    Async,
}
