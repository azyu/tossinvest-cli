use serde_json::Value;

use crate::Result;
use crate::client::TossClient;
use crate::transport::Transport;

pub async fn stocks<T: Transport>(client: &TossClient<T>, symbols: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/stocks",
            vec![("symbols".to_string(), symbols.to_string())],
            false,
        )
        .await
}

pub async fn warnings<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client
        .get_json(
            &format!("/api/v1/stocks/{symbol}/warnings"),
            Vec::new(),
            false,
        )
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;

    use super::{stocks, warnings};
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
    async fn routes_stock_info_requests() {
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
        ]));
        let client = client(requests.clone(), responses);

        stocks(&client, "AAPL,MSFT").await.unwrap();
        warnings(&client, "AAPL").await.unwrap();

        let captured = requests.lock();
        assert_eq!(captured.len(), 3);
        assert_eq!(captured[1].path, "/api/v1/stocks");
        assert_eq!(
            captured[1].query,
            vec![("symbols".to_string(), "AAPL,MSFT".to_string())]
        );
        assert_eq!(captured[2].path, "/api/v1/stocks/AAPL/warnings");
        assert!(captured[2].query.is_empty());
    }
}
