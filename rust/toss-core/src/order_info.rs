use crate::Result;
use crate::client::TossClient;
use crate::models::order_info::{BuyingPowerResponse, Commission, SellableQuantityResponse};
use crate::transport::Transport;

pub async fn buying_power<T: Transport>(
    client: &TossClient<T>,
    currency: &str,
) -> Result<BuyingPowerResponse> {
    client
        .get_typed(
            "/api/v1/buying-power",
            vec![("currency".to_string(), currency.to_string())],
            true,
        )
        .await
}

pub async fn sellable_quantity<T: Transport>(
    client: &TossClient<T>,
    symbol: &str,
) -> Result<SellableQuantityResponse> {
    client
        .get_typed(
            "/api/v1/sellable-quantity",
            vec![("symbol".to_string(), symbol.to_string())],
            true,
        )
        .await
}

pub async fn commissions<T: Transport>(client: &TossClient<T>) -> Result<Vec<Commission>> {
    client
        .get_typed("/api/v1/commissions", Vec::new(), true)
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use async_trait::async_trait;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::{buying_power, commissions, sellable_quantity};
    use crate::auth::TokenManager;
    use crate::client::TossClient;
    use crate::config::AppConfig;
    use crate::models::order_info::{BuyingPowerResponse, Commission, SellableQuantityResponse};
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
        let dir = std::env::temp_dir().join(format!("toss-core-order-info-{name}-{unique}"));
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

    #[tokio::test]
    async fn buying_power_calls_buying_power_path_with_account_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": {
                        "currency": "USD",
                        "cashBuyingPower": "3500.5"
                    }
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests.clone(), responses, "buying-power");

        let result: BuyingPowerResponse = buying_power(&client, "USD").await.expect("buying power");

        let captured = requests.lock();
        assert_eq!(
            captured.len(),
            2,
            "expected token fetch and order-info request"
        );
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/api/v1/buying-power");
        assert_eq!(
            request.query,
            vec![("currency".to_string(), "USD".to_string())]
        );
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
        assert_eq!(result.currency.0, "USD");
        assert_eq!(result.cash_buying_power, "3500.5");
    }

    #[tokio::test]
    async fn sellable_quantity_calls_sellable_quantity_path_with_account_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": {
                        "sellableQuantity": "5.5"
                    }
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests.clone(), responses, "sellable-quantity");

        let result: SellableQuantityResponse = sellable_quantity(&client, "AAPL")
            .await
            .expect("sellable quantity");

        let captured = requests.lock();
        assert_eq!(
            captured.len(),
            2,
            "expected token fetch and order-info request"
        );
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/api/v1/sellable-quantity");
        assert_eq!(
            request.query,
            vec![("symbol".to_string(), "AAPL".to_string())]
        );
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
        assert_eq!(result.sellable_quantity, "5.5");
    }

    #[tokio::test]
    async fn commissions_calls_commissions_path_with_account_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": [
                        {
                            "marketCountry": "KR",
                            "commissionRate": "0.015",
                            "startDate": "2026-01-01",
                            "endDate": "2026-12-31"
                        },
                        {
                            "marketCountry": "US",
                            "commissionRate": "0.1",
                            "startDate": null,
                            "endDate": "2026-06-30"
                        }
                    ]
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests.clone(), responses, "commissions");

        let result: Vec<Commission> = commissions(&client).await.expect("commissions");

        let captured = requests.lock();
        assert_eq!(
            captured.len(),
            2,
            "expected token fetch and order-info request"
        );
        let request = &captured[1];
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/api/v1/commissions");
        assert!(request.query.is_empty());
        assert_eq!(account_header(request), Some("42"));
        assert_eq!(authorization_header(request), Some("Bearer token-1"));
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].market_country.0, "KR");
        assert_eq!(result[0].commission_rate, "0.015");
        assert_eq!(result[0].start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(result[0].end_date.as_deref(), Some("2026-12-31"));
        assert_eq!(result[1].market_country.0, "US");
        assert_eq!(result[1].commission_rate, "0.1");
        assert_eq!(result[1].start_date, None);
        assert_eq!(result[1].end_date.as_deref(), Some("2026-06-30"));
    }
}
