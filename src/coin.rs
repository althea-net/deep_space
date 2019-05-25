use num256::Uint256;

/// Coin holds some amount of one currency
#[derive(Serialize, Debug, Default)]
pub struct Coin {
    pub denom: String,
    pub amount: Uint256,
}
