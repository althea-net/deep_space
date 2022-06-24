use crate::address::Address;
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::cosmos::tx::v1beta1::Fee as ProtoFee;
use num256::Uint256;
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

/// Coin holds some amount of one currency we convert from ProtoCoin to do more
/// validation and provide a generally nicer interface
#[derive(Serialize, Debug, Default, Clone, Deserialize, Eq, PartialEq, Hash)]
pub struct Coin {
    pub amount: Uint256,
    pub denom: String,
}

impl fmt::Display for Coin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.denom)
    }
}

impl TryFrom<&str> for Coin {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for Coin {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        let mut split_idx = 0;
        for (idx, char) in value.char_indices() {
            if char.is_alphabetic() {
                split_idx = idx;
                break;
            }
        }
        let (amount, denom) = value.split_at(split_idx);
        match amount.parse() {
            Ok(v) => Ok(Coin {
                amount: v,
                denom: denom.to_string(),
            }),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Coin {
    pub fn new(amount: Uint256, denom: String) -> Coin {
        Coin { amount, denom }
    }

    /// utility function to display a list of coins
    pub fn display_list(input: &[Coin]) -> String {
        let mut out = String::new();
        for i in input {
            out += &i.to_string()
        }
        out
    }
}

impl From<ProtoCoin> for Coin {
    fn from(value: ProtoCoin) -> Self {
        Coin {
            denom: value.denom,
            amount: value.amount.parse().unwrap(),
        }
    }
}

impl From<Coin> for ProtoCoin {
    fn from(value: Coin) -> Self {
        ProtoCoin {
            denom: value.denom,
            amount: value.amount.to_string(),
        }
    }
}

/// Fee represents everything about a Cosmos transaction fee, including the gas limit
/// who pays, and how much of an arbitrary number of Coin structs.
#[derive(Serialize, Debug, Default, Clone, Deserialize, Eq, PartialEq, Hash)]
pub struct Fee {
    pub amount: Vec<Coin>,
    pub gas_limit: u64,
    pub payer: Option<Address>,
    pub granter: Option<String>,
}

impl From<ProtoFee> for Fee {
    fn from(value: ProtoFee) -> Self {
        let mut converted_coins = Vec::new();
        for coin in value.amount {
            converted_coins.push(coin.into());
        }
        let payer = if let Ok(addr) = value.payer.parse() {
            Some(addr)
        } else {
            None
        };
        let granter = if value.granter.is_empty() {
            None
        } else {
            Some(value.granter)
        };
        Fee {
            amount: converted_coins,
            gas_limit: value.gas_limit,
            payer,
            granter,
        }
    }
}

impl From<Fee> for ProtoFee {
    fn from(value: Fee) -> Self {
        let mut converted_coins = Vec::new();
        for coin in value.amount {
            converted_coins.push(coin.into());
        }
        let payer = if let Some(s) = value.payer {
            s.to_string()
        } else {
            String::new()
        };
        let granter = if let Some(v) = value.granter {
            v
        } else {
            String::new()
        };
        ProtoFee {
            amount: converted_coins,
            gas_limit: value.gas_limit,
            payer,
            granter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CosmosPrivateKey, PrivateKey};

    #[test]
    fn test_coin_parse() {
        let _test: Coin = "100footoken".parse().unwrap();
        let _test2: Coin = "100000000000gravity0x7580bFE88Dd3d07947908FAE12d95872a260F2D8"
            .parse()
            .unwrap();
        let _test3: Coin = "100000000000gravity0xD7600ae27C99988A6CD360234062b540F88ECA43"
            .parse()
            .unwrap();

        let _res = CosmosPrivateKey::from_phrase("swim cereal address police kiwi ship safe raven other place lizard index auction mother arrive sad void real library upgrade chase frequent bike diesel", "").unwrap();
    }
}
