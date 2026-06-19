use serde::{Deserialize, Serialize};

use super::common::{Currency, MoneyValue, QuantityValue};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceResponse {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub last_price: MoneyValue,
    pub currency: Currency,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookEntry {
    pub price: MoneyValue,
    pub volume: QuantityValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub currency: Currency,
    pub asks: Vec<OrderbookEntry>,
    pub bids: Vec<OrderbookEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub price: MoneyValue,
    pub volume: QuantityValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub currency: Currency,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceLimitResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upper_limit_price: Option<MoneyValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower_limit_price: Option<MoneyValue>,
    pub currency: Currency,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandlePageResponse {
    pub candles: Vec<Candle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_before: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub open_price: MoneyValue,
    pub high_price: MoneyValue,
    pub low_price: MoneyValue,
    pub close_price: MoneyValue,
    pub volume: QuantityValue,
    pub currency: Currency,
}
