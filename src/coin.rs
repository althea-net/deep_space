use num256::Uint256;

/// Coin holds some amount of one currency
pub struct Coin {
    denom: String,
    amount: Uint256,
}
