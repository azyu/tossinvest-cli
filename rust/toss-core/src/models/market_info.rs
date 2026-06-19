use serde::{Deserialize, Serialize};

use super::common::Currency;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RateChangeType(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRateResponse {
    pub base_currency: Currency,
    pub quote_currency: Currency,
    pub rate: String,
    pub mid_rate: String,
    pub basis_point: String,
    pub rate_change_type: RateChangeType,
    pub valid_from: String,
    pub valid_until: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreMarketSession {
    pub start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_price_auction_start_time: Option<String>,
    pub end_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegularMarketSession {
    pub start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_price_auction_start_time: Option<String>,
    pub end_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterMarketSession {
    pub start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_price_auction_end_time: Option<String>,
    pub end_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegratedHour {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_market: Option<PreMarketSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regular_market: Option<RegularMarketSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_market: Option<AfterMarketSession>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KrMarketDay {
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrated: Option<IntegratedHour>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsDayMarketSession {
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsPreMarketSession {
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsRegularMarketSession {
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsAfterMarketSession {
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsMarketDay {
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_market: Option<UsDayMarketSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_market: Option<UsPreMarketSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regular_market: Option<UsRegularMarketSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_market: Option<UsAfterMarketSession>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KrMarketCalendarResponse {
    pub today: KrMarketDay,
    pub previous_business_day: KrMarketDay,
    pub next_business_day: KrMarketDay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsMarketCalendarResponse {
    pub today: UsMarketDay,
    pub previous_business_day: UsMarketDay,
    pub next_business_day: UsMarketDay,
}
