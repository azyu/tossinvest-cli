use serde::{Deserialize, Serialize};

use super::common::{Currency, MarketCountry, MoneyValue, QuantityValue};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceBreakdown {
    pub krw: MoneyValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usd: Option<MoneyValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketValue {
    pub purchase_amount: MoneyValue,
    pub amount: MoneyValue,
    pub amount_after_cost: MoneyValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfitLoss {
    pub amount: MoneyValue,
    pub amount_after_cost: MoneyValue,
    pub rate: String,
    pub rate_after_cost: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyProfitLoss {
    pub amount: MoneyValue,
    pub rate: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub commission: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewMarketValue {
    pub amount: PriceBreakdown,
    pub amount_after_cost: PriceBreakdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewProfitLoss {
    pub amount: PriceBreakdown,
    pub amount_after_cost: PriceBreakdown,
    pub rate: String,
    pub rate_after_cost: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewDailyProfitLoss {
    pub amount: PriceBreakdown,
    pub rate: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsItem {
    pub symbol: String,
    pub name: String,
    pub market_country: MarketCountry,
    pub currency: Currency,
    pub quantity: QuantityValue,
    pub last_price: MoneyValue,
    pub average_purchase_price: MoneyValue,
    pub market_value: MarketValue,
    pub profit_loss: ProfitLoss,
    pub daily_profit_loss: DailyProfitLoss,
    pub cost: Cost,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsOverview {
    pub total_purchase_amount: PriceBreakdown,
    pub market_value: OverviewMarketValue,
    pub profit_loss: OverviewProfitLoss,
    pub daily_profit_loss: OverviewDailyProfitLoss,
    pub items: Vec<HoldingsItem>,
}
