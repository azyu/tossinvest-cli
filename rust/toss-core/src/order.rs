use serde::Serialize;

use crate::Result;
use crate::client::TossClient;
use crate::models::order::{
    OrderCreateRequest, OrderHistoryListResponse, OrderHistoryOrder, OrderModifyRequest,
    OrderOperationResponse, OrderResponse,
};
use crate::transport::{HttpRequest, Transport};

#[derive(Serialize)]
struct EmptyRequest {}

pub async fn create<T: Transport>(
    client: &TossClient<T>,
    request: &OrderCreateRequest,
) -> Result<OrderResponse> {
    client.post_typed("/api/v1/orders", request, true).await
}

pub async fn modify<T: Transport>(
    client: &TossClient<T>,
    order_id: &str,
    request: &OrderModifyRequest,
) -> Result<OrderOperationResponse> {
    client
        .post_typed(&format!("/api/v1/orders/{order_id}/modify"), request, true)
        .await
}

pub async fn cancel<T: Transport>(
    client: &TossClient<T>,
    order_id: &str,
) -> Result<OrderOperationResponse> {
    client
        .post_typed(
            &format!("/api/v1/orders/{order_id}/cancel"),
            &EmptyRequest {},
            true,
        )
        .await
}

pub async fn list<T: Transport>(
    client: &TossClient<T>,
    query: Vec<(String, String)>,
) -> Result<OrderHistoryListResponse> {
    client.get_typed("/api/v1/orders", query, true).await
}

pub async fn show<T: Transport>(
    client: &TossClient<T>,
    order_id: &str,
) -> Result<OrderHistoryOrder> {
    client
        .get_typed(&format!("/api/v1/orders/{order_id}"), Vec::new(), true)
        .await
}

pub async fn build_create_dry_run<T: Transport>(
    client: &TossClient<T>,
    request: &OrderCreateRequest,
) -> Result<HttpRequest> {
    client
        .build_post_request("/api/v1/orders", request, true, false)
        .await
}

pub async fn build_modify_dry_run<T: Transport>(
    client: &TossClient<T>,
    order_id: &str,
    request: &OrderModifyRequest,
) -> Result<HttpRequest> {
    client
        .build_post_request(
            &format!("/api/v1/orders/{order_id}/modify"),
            request,
            true,
            false,
        )
        .await
}

pub async fn build_cancel_dry_run<T: Transport>(
    client: &TossClient<T>,
    order_id: &str,
) -> Result<HttpRequest> {
    client
        .build_post_request(
            &format!("/api/v1/orders/{order_id}/cancel"),
            &EmptyRequest {},
            true,
            false,
        )
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use async_trait::async_trait;
    use parking_lot::Mutex;
    use serde_json::{Value, json};

    use super::{
        build_cancel_dry_run, build_create_dry_run, build_modify_dry_run, cancel, create, list,
        modify, show,
    };
    use crate::auth::TokenManager;
    use crate::client::TossClient;
    use crate::config::AppConfig;
    use crate::models::order::{
        OrderCreateRequest, OrderModifyRequest, OrderSide, OrderType, TimeInForce,
    };
    use crate::transport::{HttpMethod, HttpRequest, HttpResponse, Transport};

    #[derive(Clone)]
    struct QueueTransport {
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        responses: Arc<Mutex<Vec<HttpResponse>>>,
    }

    #[async_trait]
    impl Transport for QueueTransport {
        async fn send(&self, request: HttpRequest) -> crate::Result<HttpResponse> {
            self.requests.lock().push(request);
            let mut responses = self.responses.lock();
            assert!(
                !responses.is_empty(),
                "test transport response queue exhausted"
            );
            Ok(responses.remove(0))
        }
    }

    fn unique_cache_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("toss-core-order-{name}-{unique}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join("token-cache.json")
    }

    fn client(
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        responses: Arc<Mutex<Vec<HttpResponse>>>,
        name: &str,
    ) -> TossClient<QueueTransport> {
        let transport = QueueTransport {
            requests,
            responses,
        };
        let token_manager = TokenManager::new_with_cache_path(
            "client-id".to_string(),
            "client-secret".to_string(),
            unique_cache_path(name),
            transport.clone(),
        );
        TossClient::new_with_parts(
            AppConfig {
                client_id: "client-id".to_string(),
                client_secret: "client-secret".to_string(),
                account_seq: Some(42),
            },
            token_manager,
            transport,
        )
    }

    fn token_response() -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#
                .to_vec(),
        }
    }

    fn order_response(order_id: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: json!({
                "result": {
                    "orderId": order_id,
                    "clientOrderId": "order-001"
                }
            })
            .to_string()
            .into_bytes(),
        }
    }

    fn account_header(request: &HttpRequest) -> Option<&str> {
        request
            .headers
            .iter()
            .find(|header| header.name == "X-Tossinvest-Account")
            .map(|header| header.value.as_str())
    }

    fn authorization_header(request: &HttpRequest) -> Option<&str> {
        request
            .headers
            .iter()
            .find(|header| header.name == "authorization")
            .map(|header| header.value.as_str())
    }

    fn request_body_json(request: &HttpRequest) -> Value {
        serde_json::from_slice(request.body.as_ref().expect("request body"))
            .expect("valid json body")
    }

    fn create_request() -> OrderCreateRequest {
        OrderCreateRequest {
            client_order_id: Some("order-001".to_string()),
            symbol: "005930".to_string(),
            side: OrderSide::BUY,
            order_type: OrderType::LIMIT,
            time_in_force: Some(TimeInForce::DAY),
            quantity: Some("10".to_string()),
            price: Some("70000".to_string()),
            confirm_high_value_order: Some(true),
            order_amount: None,
        }
    }

    fn modify_request() -> OrderModifyRequest {
        OrderModifyRequest {
            order_type: OrderType::LIMIT,
            quantity: Some("15".to_string()),
            price: Some("71000".to_string()),
            confirm_high_value_order: Some(true),
        }
    }

    #[tokio::test]
    async fn create_posts_typed_body_with_account_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            order_response("create-order"),
        ]));
        let client = client(requests.clone(), responses, "create-live");

        let result = create(&client, &create_request())
            .await
            .expect("create order");
        assert_eq!(result.order_id, "create-order");

        let captured = requests.lock();
        assert_eq!(captured.len(), 2, "expected token fetch and order request");
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.path, "/api/v1/orders");
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
        assert_eq!(
            request_body_json(request),
            json!({
                "clientOrderId": "order-001",
                "symbol": "005930",
                "side": "BUY",
                "orderType": "LIMIT",
                "timeInForce": "DAY",
                "quantity": "10",
                "price": "70000",
                "confirmHighValueOrder": true
            })
        );
    }

    #[tokio::test]
    async fn modify_posts_typed_body_to_modify_path() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            order_response("modify-order"),
        ]));
        let client = client(requests.clone(), responses, "modify-live");

        let result = modify(&client, "order-123", &modify_request())
            .await
            .expect("modify order");
        assert_eq!(result.order_id, "modify-order");

        let captured = requests.lock();
        assert_eq!(captured.len(), 2, "expected token fetch and order request");
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.path, "/api/v1/orders/order-123/modify");
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
        assert_eq!(
            request_body_json(request),
            json!({
                "orderType": "LIMIT",
                "quantity": "15",
                "price": "71000",
                "confirmHighValueOrder": true
            })
        );
    }

    #[tokio::test]
    async fn cancel_posts_empty_body_to_cancel_path() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            order_response("cancel-order"),
        ]));
        let client = client(requests.clone(), responses, "cancel-live");

        let result = cancel(&client, "order-123").await.expect("cancel order");
        assert_eq!(result.order_id, "cancel-order");

        let captured = requests.lock();
        assert_eq!(captured.len(), 2, "expected token fetch and order request");
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.path, "/api/v1/orders/order-123/cancel");
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
        assert_eq!(request_body_json(request), json!({}));
    }

    #[tokio::test]
    async fn build_create_dry_run_omits_authorization_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(Vec::new()));
        let client = client(requests.clone(), responses, "create-dry-run");

        let request = build_create_dry_run(&client, &create_request())
            .await
            .expect("build create dry run");

        assert_eq!(
            requests.lock().len(),
            0,
            "dry-run builder must not fetch token"
        );
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.path, "/api/v1/orders");
        assert_eq!(account_header(&request), Some("42"));
        assert_eq!(authorization_header(&request), None);
        assert_eq!(
            request_body_json(&request),
            json!({
                "clientOrderId": "order-001",
                "symbol": "005930",
                "side": "BUY",
                "orderType": "LIMIT",
                "timeInForce": "DAY",
                "quantity": "10",
                "price": "70000",
                "confirmHighValueOrder": true
            })
        );
    }

    #[tokio::test]
    async fn build_modify_dry_run_omits_authorization_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(Vec::new()));
        let client = client(requests.clone(), responses, "modify-dry-run");

        let request = build_modify_dry_run(&client, "order-123", &modify_request())
            .await
            .expect("build modify dry run");

        assert_eq!(
            requests.lock().len(),
            0,
            "dry-run builder must not fetch token"
        );
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.path, "/api/v1/orders/order-123/modify");
        assert_eq!(account_header(&request), Some("42"));
        assert_eq!(authorization_header(&request), None);
        assert_eq!(
            request_body_json(&request),
            json!({
                "orderType": "LIMIT",
                "quantity": "15",
                "price": "71000",
                "confirmHighValueOrder": true
            })
        );
    }

    #[tokio::test]
    async fn build_cancel_dry_run_omits_authorization_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(Vec::new()));
        let client = client(requests.clone(), responses, "cancel-dry-run");

        let request = build_cancel_dry_run(&client, "order-123")
            .await
            .expect("build cancel dry run");

        assert_eq!(
            requests.lock().len(),
            0,
            "dry-run builder must not fetch token"
        );
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.path, "/api/v1/orders/order-123/cancel");
        assert_eq!(account_header(&request), Some("42"));
        assert_eq!(authorization_header(&request), None);
        assert_eq!(request_body_json(&request), json!({}));
    }
    #[tokio::test]
    async fn list_gets_open_orders_with_account_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": {
                        "orders": [
                            {
                                "orderId": "order-123",
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
                    }
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests.clone(), responses, "list-live");

        let result = list(&client, vec![("status".to_string(), "OPEN".to_string())])
            .await
            .expect("list orders");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].order_id, "order-123");

        let captured = requests.lock();
        assert_eq!(captured.len(), 2, "expected token fetch and order request");
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/api/v1/orders");
        assert_eq!(
            request.query,
            vec![("status".to_string(), "OPEN".to_string())]
        );
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
    }

    #[tokio::test]
    async fn show_gets_order_detail_with_account_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": {
                        "orderId": "order-123",
                        "symbol": "AAPL",
                        "side": "BUY",
                        "orderType": "LIMIT",
                        "timeInForce": "DAY",
                        "status": "FILLED",
                        "price": "180",
                        "quantity": "1",
                        "orderAmount": null,
                        "currency": "USD",
                        "orderedAt": "2026-03-29T09:30:00+09:00",
                        "canceledAt": null,
                        "execution": {
                            "filledQuantity": "1",
                            "averageFilledPrice": "180",
                            "filledAmount": "180",
                            "commission": "0.01",
                            "tax": "0",
                            "filledAt": "2026-03-29T09:30:05+09:00",
                            "settlementDate": null
                        }
                    }
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests.clone(), responses, "show-live");

        let result = show(&client, "order-123").await.expect("show order");
        assert_eq!(result.order_id, "order-123");
        assert_eq!(result.status, "FILLED");
        assert_eq!(result.execution.filled_quantity, "1");

        let captured = requests.lock();
        assert_eq!(captured.len(), 2, "expected token fetch and order request");
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/api/v1/orders/order-123");
        assert!(request.query.is_empty());
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
    }
}
