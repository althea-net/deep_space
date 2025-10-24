// Type URLs for common Msg implementations

// cosmos-sdk msgs
pub const MSG_SEND_TYPE_URL: &str = "/cosmos.bank.v1beta1.MsgSend";

pub const MSG_VERIFY_INVARIANT_TYPE_URL: &str = "/cosmos.crisis.v1beta1.MsgVerifyInvariant";

pub const SECP256K1_PUBKEY_TYPE_URL: &str = "/cosmos.crypto.secp256k1.PubKey";

pub const MSG_FUND_COMMUNITY_POOL_TYPE_URL: &str =
    "/cosmos.distribution.v1beta1.MsgFundCommunityPool";
pub const MSG_WITHDRAW_DELEGATOR_REWARD_TYPE_URL: &str =
    "/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward";
pub const MSG_WITHDRAW_VALIDATOR_COMMISSION_TYPE_URL: &str =
    "/cosmos.distribution.v1beta1.MsgWithdrawValidatorCommission";

pub const LEGACY_MSG_SUBMIT_PROPOSAL_TYPE_URL: &str = "/cosmos.gov.v1beta1.MsgSubmitProposal";
pub const LEGACY_MSG_VOTE_TYPE_URL: &str = "/cosmos.gov.v1beta1.MsgVote";
pub const MSG_SUBMIT_PROPOSAL_TYPE_URL: &str = "/cosmos.gov.v1.MsgSubmitProposal";
pub const MSG_VOTE_TYPE_URL: &str = "/cosmos.gov.v1.MsgVote";

pub const MSG_BEGIN_REDELEGATE_TYPE_URL: &str = "/cosmos.staking.v1beta1.MsgBeginRedelegate";
pub const MSG_DELEGATE_TYPE_URL: &str = "/cosmos.staking.v1beta1.MsgDelegate";
pub const MSG_UNDELEGATE_TYPE_URL: &str = "/cosmos.staking.v1beta1.MsgUndelegate";

// ibc msgs
pub const MSG_TRANSFER_TYPE_URL: &str = "/ibc.applications.transfer.v1.MsgTransfer";

// cosmos-sdk proposals
pub const PARAMETER_CHANGE_PROPOSAL_TYPE_URL: &str =
    "/cosmos.params.v1beta1.ParameterChangeProposal";
pub const SOFTWARE_UPGRADE_PROPOSAL_TYPE_URL: &str =
    "/cosmos.upgrade.v1beta1.SoftwareUpgradeProposal";

// althea msgs
pub const MSG_MICROTX_TYPE_URL: &str = "/althea.microtx.v1.MsgMicrotx";

// althea proposals
pub const REGISTER_COIN_PROPOSAL_TYPE_URL: &str = "/althea.althea.v1.RegisterCoinProposal";
pub const REGISTER_ERC20_PROPOSAL_TYPE_URL: &str = "/althea.althea.v1.RegisterERC20Proposal";
