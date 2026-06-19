use serde_json::Value;

use crate::Result;
use crate::client::TossClient;
use crate::transport::Transport;

pub async fn holdings<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/holdings", Vec::new(), true).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;

    use super::holdings;
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

        holdings(&client).await.unwrap();

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
}
