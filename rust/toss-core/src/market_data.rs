use serde_json::Value;

use crate::Result;
use crate::client::TossClient;
use crate::transport::Transport;

pub async fn prices<T: Transport>(client: &TossClient<T>, symbols: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/prices",
            vec![("symbols".to_string(), symbols.to_string())],
            false,
        )
        .await
}

pub async fn orderbook<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/orderbook",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn trades<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/trades",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn price_limits<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/price-limits",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn candles<T: Transport>(
    client: &TossClient<T>,
    query: Vec<(String, String)>,
) -> Result<Value> {
    client.get_json("/api/v1/candles", query, false).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;

    use super::{candles, orderbook, price_limits, prices, trades};
    use crate::auth::TokenManager;
    use crate::client::TossClient;
    use crate::config::AppConfig;
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
                account_seq: None,
            },
            token_manager,
            transport,
        )
    }

    #[tokio::test]
    async fn routes_market_data_requests() {
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
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"result":{}}"#.to_vec(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"result":{}}"#.to_vec(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"result":{}}"#.to_vec(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"result":{}}"#.to_vec(),
            },
        ]));
        let client = client(requests.clone(), responses);

        prices(&client, "AAPL,MSFT").await.unwrap();
        orderbook(&client, "AAPL").await.unwrap();
        trades(&client, "AAPL").await.unwrap();
        price_limits(&client, "AAPL").await.unwrap();
        candles(
            &client,
            vec![
                ("symbol".to_string(), "AAPL".to_string()),
                ("interval".to_string(), "1d".to_string()),
            ],
        )
        .await
        .unwrap();

        let captured = requests.lock();
        assert_eq!(captured.len(), 6);
        assert_eq!(captured[1].path, "/api/v1/prices");
        assert_eq!(
            captured[1].query,
            vec![("symbols".to_string(), "AAPL,MSFT".to_string())]
        );
        assert_eq!(captured[2].path, "/api/v1/orderbook");
        assert_eq!(
            captured[2].query,
            vec![("symbol".to_string(), "AAPL".to_string())]
        );
        assert_eq!(captured[3].path, "/api/v1/trades");
        assert_eq!(captured[4].path, "/api/v1/price-limits");
        assert_eq!(captured[5].path, "/api/v1/candles");
        assert_eq!(
            captured[5].query,
            vec![
                ("symbol".to_string(), "AAPL".to_string()),
                ("interval".to_string(), "1d".to_string())
            ]
        );
    }
}
