use num256::Uint256;

/// Coin holds some amount of one currency
#[derive(Serialize, Debug, Default, Clone, Deserialize, Eq, PartialEq, Hash)]
pub struct Coin {
    pub amount: Uint256,
    pub denom: String,
}

impl Coin {
    pub fn new(amount: Uint256, denom: String) -> Coin {
        Coin { amount, denom }
    }
}
