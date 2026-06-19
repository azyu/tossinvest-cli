use serde::{Deserialize, Serialize};

use super::common::AccountType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub account_no: String,
    pub account_seq: i64,
    pub account_type: AccountType,
}
