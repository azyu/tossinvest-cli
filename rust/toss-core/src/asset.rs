use serde_json::Value;

use crate::Result;
use crate::client::TossClient;
use crate::models::asset::HoldingsOverview;
use crate::transport::Transport;

pub async fn holdings_json<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/holdings", Vec::new(), true).await
}

pub async fn holdings<T: Transport>(client: &TossClient<T>) -> Result<HoldingsOverview> {
    client.get_typed("/api/v1/holdings", Vec::new(), true).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::{holdings, holdings_json};
    use crate::auth::TokenManager;
    use crate::client::TossClient;
    use crate::config::AppConfig;
    use crate::models::asset::HoldingsOverview;
    use crate::transport::{HttpRequest, HttpResponse, Transport};

    #[derive(Clone)]
    struct QueueTransport {
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        responses: Arc<Mutex<Vec<HttpResponse>>>,
    }

    #[async_trait]
    impl Transport for QueueTransport {
        async fn send(&self, request: HttpRequest) -> crate::Result<HttpResponse> {
            self.requests.lock().push(request);
            Ok(self.responses.lock().remove(0))
        }
    }

    fn client(
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        responses: Arc<Mutex<Vec<HttpResponse>>>,
    ) -> TossClient<QueueTransport> {
        let transport = QueueTransport {
            requests,
            responses,
        };
        let tempdir = tempfile::tempdir().unwrap();
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempdir.path().join("token.json"),
            transport.clone(),
        );
        TossClient::new_with_parts(
            AppConfig {
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                account_seq: Some(77),
            },
            token_manager,
            transport,
        )
    }

    #[tokio::test]
    async fn routes_holdings_request() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#
                    .to_vec(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"result":{}}"#.to_vec(),
            },
        ]));
        let client = client(requests.clone(), responses);

        holdings_json(&client).await.unwrap();

        let captured = requests.lock();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].path, "/api/v1/holdings");
        assert_eq!(
            captured[1]
                .headers
                .iter()
                .find(|h| h.name == "X-Tossinvest-Account")
                .map(|h| h.value.as_str()),
            Some("77")
        );
    }

    #[tokio::test]
    async fn deserializes_holdings_quantity_as_value() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#
                    .to_vec(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: serde_json::json!({
                    "result": {
                        "totalPurchaseAmount": {"krw": "6500000", "usd": "1553"},
                        "marketValue": {
                            "amount": {"krw": "7200000", "usd": "1785"},
                            "amountAfterCost": {"krw": "7050000", "usd": "1771.43"}
                        },
                        "profitLoss": {
                            "amount": {"krw": "700000", "usd": "232"},
                            "amountAfterCost": {"krw": "550000", "usd": "218.43"},
                            "rate": "0.1179",
                            "rateAfterCost": "0.0983"
                        },
                        "dailyProfitLoss": {
                            "amount": {"krw": "100000", "usd": "25"},
                            "rate": "0.0141"
                        },
                        "items": [
                            {
                                "symbol": "AAPL",
                                "name": "Apple",
                                "marketCountry": "US",
                                "currency": "USD",
                                "quantity": "100.5",
                                "lastPrice": "185.75",
                                "averagePurchasePrice": "172.10",
                                "marketValue": {
                                    "purchaseAmount": "17210",
                                    "amount": "18718.88",
                                    "amountAfterCost": "18680.88"
                                },
                                "profitLoss": {
                                    "amount": "1508.88",
                                    "amountAfterCost": "1470.88",
                                    "rate": "0.0876",
                                    "rateAfterCost": "0.0854"
                                },
                                "dailyProfitLoss": {
                                    "amount": "12.25",
                                    "rate": "0.0066"
                                },
                                "cost": {
                                    "commission": "18.50",
                                    "tax": null
                                }
                            }
                        ]
                    }
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests, responses);

        let holdings: HoldingsOverview = holdings(&client).await.unwrap();

        assert_eq!(holdings.items.len(), 1);
        assert_eq!(holdings.items[0].quantity, json!("100.5"));
    }
}
