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
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_high_value_order: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_amount: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderModifyRequest {
    pub order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderOperationResponse {
    pub order_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderHistoryExecution {
    pub filled_quantity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_filled_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commission: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_date: Option<String>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderHistoryOrder {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    pub quantity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_amount: Option<String>,
    pub currency: String,
    pub ordered_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_at: Option<String>,
    pub execution: OrderHistoryExecution,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderHistoryListResponse {
    pub orders: Vec<OrderHistoryOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_next: bool,
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
            quantity: Some("1".to_string()),
            price: Some("181.23".to_string()),
            confirm_high_value_order: Some(true),
            order_amount: Some("181.23".to_string()),
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
            quantity: Some("2".to_string()),
            price: Some("180.00".to_string()),
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

    #[test]
    fn deserializes_order_history_order_broadly() {
        let response: OrderHistoryOrder = serde_json::from_value(json!({
            "orderId": "order-1",
            "symbol": "AAPL",
            "side": "BUY",
            "orderType": "LIMIT",
            "timeInForce": "DAY",
            "status": "OPEN",
            "price": "180",
            "quantity": "1",
            "orderAmount": null,
            "currency": "USD",
            "orderedAt": "2026-03-29T09:30:00+09:00",
            "canceledAt": null,
            "execution": {
                "filledQuantity": "0",
                "averageFilledPrice": null,
                "filledAmount": null,
                "commission": null,
                "tax": null,
                "filledAt": null,
                "settlementDate": null
            }
        }))
        .unwrap();

        assert_eq!(response.order_id, "order-1");
        assert_eq!(response.status, "OPEN");
        assert_eq!(response.execution.filled_quantity, "0");
    }

    #[test]
    fn deserializes_order_history_list_response_broadly() {
        let response: OrderHistoryListResponse = serde_json::from_value(json!({
            "orders": [
                {
                    "orderId": "order-1",
                    "symbol": "AAPL",
                    "side": "BUY",
                    "orderType": "LIMIT",
                    "timeInForce": "DAY",
                    "status": "OPEN",
                    "price": "180",
                    "quantity": "1",
                    "orderAmount": null,
                    "currency": "USD",
                    "orderedAt": "2026-03-29T09:30:00+09:00",
                    "canceledAt": null,
                    "execution": {
                        "filledQuantity": "0",
                        "averageFilledPrice": null,
                        "filledAmount": null,
                        "commission": null,
                        "tax": null,
                        "filledAt": null,
                        "settlementDate": null
                    }
                }
            ],
            "nextCursor": null,
            "hasNext": false
        }))
        .unwrap();

        assert_eq!(response.orders.len(), 1);
        assert_eq!(response.orders[0].order_id, "order-1");
        assert!(!response.has_next);
    }
}
