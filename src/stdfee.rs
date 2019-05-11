use crate::coin::Coin;

/// StdFee includes the amount of coins paid in fees and the maximum
/// gas to be used by the transaction. The ratio yields an effective "gasprice",
/// which must be above some miminum to be accepted into the mempool.
pub struct StdFee {
    amount: Vec<Coin>,
    gas: u64,
}
