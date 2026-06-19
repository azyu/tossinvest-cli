use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    #[serde(rename = "BUY")]
    BUY,
    #[serde(rename = "SELL")]
    SELL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "LIMIT")]
    LIMIT,
    #[serde(rename = "MARKET")]
    MARKET,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    #[serde(rename = "DAY")]
    DAY,
    #[serde(rename = "CLS")]
    CLS,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_high_value_order: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_amount: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderModifyRequest {
    pub order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_high_value_order: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_high_value_order: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_amount: Option<Value>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_create_request_fields_exactly() {
        let request = OrderCreateRequest {
            client_order_id: Some("client-1".to_string()),
            symbol: "AAPL".to_string(),
            side: OrderSide::BUY,
            order_type: OrderType::LIMIT,
            time_in_force: Some(TimeInForce::DAY),
            quantity: Some(json!("1")),
            price: Some(json!("181.23")),
            confirm_high_value_order: Some(true),
            order_amount: Some(json!("181.23")),
        };

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({
                "clientOrderId": "client-1",
                "symbol": "AAPL",
                "side": "BUY",
                "orderType": "LIMIT",
                "timeInForce": "DAY",
                "quantity": "1",
                "price": "181.23",
                "confirmHighValueOrder": true,
                "orderAmount": "181.23"
            })
        );
    }

    #[test]
    fn serializes_modify_request_fields_exactly() {
        let request = OrderModifyRequest {
            order_type: OrderType::MARKET,
            quantity: Some(json!("2")),
            price: Some(json!("180.00")),
            confirm_high_value_order: None,
        };

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({
                "orderType": "MARKET",
                "quantity": "2",
                "price": "180.00"
            })
        );
    }

    #[test]
    fn serializes_enums_as_toss_values() {
        assert_eq!(serde_json::to_value(OrderSide::BUY).unwrap(), json!("BUY"));
        assert_eq!(
            serde_json::to_value(OrderSide::SELL).unwrap(),
            json!("SELL")
        );
        assert_eq!(
            serde_json::to_value(OrderType::LIMIT).unwrap(),
            json!("LIMIT")
        );
        assert_eq!(
            serde_json::to_value(OrderType::MARKET).unwrap(),
            json!("MARKET")
        );
        assert_eq!(
            serde_json::to_value(TimeInForce::DAY).unwrap(),
            json!("DAY")
        );
        assert_eq!(
            serde_json::to_value(TimeInForce::CLS).unwrap(),
            json!("CLS")
        );
    }

    #[test]
    fn deserializes_order_response_broadly() {
        let response: OrderResponse = serde_json::from_value(json!({
            "orderId": "order-1",
            "clientOrderId": "client-1",
            "symbol": "AAPL",
            "side": "BUY",
            "orderType": "LIMIT",
            "timeInForce": "DAY",
            "quantity": "1",
            "price": "181.23",
            "orderAmount": "181.23"
        }))
        .unwrap();

        assert_eq!(response.order_id, "order-1");
    }
}
