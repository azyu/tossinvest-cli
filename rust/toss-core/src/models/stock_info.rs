use serde::{Deserialize, Serialize};

use super::common::Currency;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StockMarket(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecurityType(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StockStatus(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WarningType(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KrMarketDetail {
    pub liquidation_trading: bool,
    pub nxt_supported: bool,
    pub krx_trading_suspended: bool,
    #[serde(rename = "nxtTradingSuspended")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nxt_trading_suspended: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockInfo {
    pub symbol: String,
    pub name: String,
    pub english_name: String,
    pub isin_code: String,
    pub market: StockMarket,
    pub security_type: SecurityType,
    pub is_common_share: bool,
    pub status: StockStatus,
    pub currency: Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delist_date: Option<String>,
    pub shares_outstanding: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leverage_factor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub korean_market_detail: Option<KrMarketDetail>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockWarning {
    pub warning_type: WarningType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
}
