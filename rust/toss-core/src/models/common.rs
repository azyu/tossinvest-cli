use serde::{Deserialize, Serialize};

pub type MoneyValue = serde_json::Value;
pub type QuantityValue = serde_json::Value;
pub type DateString = String;
pub type TimestampString = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Currency(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MarketCountry(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountType(pub String);
