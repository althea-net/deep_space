use crate::coin::Coin;
use num256::Uint256;

/// StdFee includes the amount of coins paid in fees and the maximum
/// gas to be used by the transaction. The ratio yields an effective "gasprice",
/// which must be above some miminum to be accepted into the mempool.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct StdFee {
    pub amount: Vec<Coin>,
    pub gas: Uint256,
}
