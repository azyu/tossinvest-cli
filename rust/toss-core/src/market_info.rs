use serde_json::Value;

use crate::client::TossClient;
use crate::transport::Transport;
use crate::Result;

pub async fn exchange_rate<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/exchange-rate", Vec::new(), false).await
}

pub async fn kr_calendar<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client
        .get_json("/api/v1/market-calendar/KR", Vec::new(), false)
        .await
}

pub async fn us_calendar<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client
        .get_json("/api/v1/market-calendar/US", Vec::new(), false)
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;

    use super::{exchange_rate, kr_calendar, us_calendar};
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
        let transport = QueueTransport { requests, responses };
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
    async fn routes_market_info_requests() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            HttpResponse { status: 200, headers: Vec::new(), body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#.to_vec() },
            HttpResponse { status: 200, headers: Vec::new(), body: br#"{"result":{}}"#.to_vec() },
            HttpResponse { status: 200, headers: Vec::new(), body: br#"{"result":{}}"#.to_vec() },
            HttpResponse { status: 200, headers: Vec::new(), body: br#"{"result":{}}"#.to_vec() },
        ]));
        let client = client(requests.clone(), responses);

        exchange_rate(&client).await.unwrap();
        kr_calendar(&client).await.unwrap();
        us_calendar(&client).await.unwrap();

        let captured = requests.lock();
        assert_eq!(captured.len(), 4);
        assert_eq!(captured[1].path, "/api/v1/exchange-rate");
        assert_eq!(captured[2].path, "/api/v1/market-calendar/KR");
        assert_eq!(captured[3].path, "/api/v1/market-calendar/US");
    }
}
