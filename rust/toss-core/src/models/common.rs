use serde::{Deserialize, Serialize};

pub type MoneyValue = String;
pub type QuantityValue = String;
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
