use serde_json::Value;

use crate::Result;
use crate::client::TossClient;
use crate::models::market_info::{
    ExchangeRateResponse, KrMarketCalendarResponse, UsMarketCalendarResponse,
};
use crate::transport::Transport;

pub async fn exchange_rate_json<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client
        .get_json("/api/v1/exchange-rate", Vec::new(), false)
        .await
}

pub async fn kr_calendar_json<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client
        .get_json("/api/v1/market-calendar/KR", Vec::new(), false)
        .await
}

pub async fn us_calendar_json<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client
        .get_json("/api/v1/market-calendar/US", Vec::new(), false)
        .await
}

pub async fn exchange_rate<T: Transport>(client: &TossClient<T>) -> Result<ExchangeRateResponse> {
    client
        .get_typed("/api/v1/exchange-rate", Vec::new(), false)
        .await
}

pub async fn kr_calendar<T: Transport>(client: &TossClient<T>) -> Result<KrMarketCalendarResponse> {
    client
        .get_typed("/api/v1/market-calendar/KR", Vec::new(), false)
        .await
}

pub async fn us_calendar<T: Transport>(client: &TossClient<T>) -> Result<UsMarketCalendarResponse> {
    client
        .get_typed("/api/v1/market-calendar/US", Vec::new(), false)
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::{
        exchange_rate, exchange_rate_json, kr_calendar, kr_calendar_json, us_calendar,
        us_calendar_json,
    };
    use crate::auth::TokenManager;
    use crate::client::TossClient;
    use crate::config::AppConfig;
    use crate::models::market_info::{
        ExchangeRateResponse, KrMarketCalendarResponse, UsMarketCalendarResponse,
    };
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
    async fn routes_market_info_requests() {
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
        ]));
        let client = client(requests.clone(), responses);

        exchange_rate_json(&client).await.unwrap();
        kr_calendar_json(&client).await.unwrap();
        us_calendar_json(&client).await.unwrap();

        let captured = requests.lock();
        assert_eq!(captured.len(), 4);
        assert_eq!(captured[1].path, "/api/v1/exchange-rate");
        assert_eq!(captured[2].path, "/api/v1/market-calendar/KR");
        assert_eq!(captured[3].path, "/api/v1/market-calendar/US");
    }

    #[tokio::test]
    async fn deserializes_typed_exchange_rate_and_calendars() {
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
                body: json!({
                    "result": {
                        "baseCurrency": "USD",
                        "quoteCurrency": "KRW",
                        "rate": "1380.5",
                        "midRate": "1375",
                        "basisPoint": "40",
                        "rateChangeType": "UP",
                        "validFrom": "2026-03-25T09:30:00+09:00",
                        "validUntil": "2026-03-25T09:31:00+09:00"
                    }
                })
                .to_string()
                .into_bytes(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": {
                        "today": {"date": "2026-03-25", "integrated": null},
                        "previousBusinessDay": {"date": "2026-03-24", "integrated": null},
                        "nextBusinessDay": {"date": "2026-03-26", "integrated": null}
                    }
                })
                .to_string()
                .into_bytes(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": {
                        "today": {
                            "date": "2026-03-25",
                            "dayMarket": null,
                            "preMarket": null,
                            "regularMarket": null,
                            "afterMarket": null
                        },
                        "previousBusinessDay": {
                            "date": "2026-03-24",
                            "dayMarket": null,
                            "preMarket": null,
                            "regularMarket": null,
                            "afterMarket": null
                        },
                        "nextBusinessDay": {
                            "date": "2026-03-26",
                            "dayMarket": null,
                            "preMarket": null,
                            "regularMarket": null,
                            "afterMarket": null
                        }
                    }
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests, responses);

        let exchange_rate: ExchangeRateResponse = exchange_rate(&client).await.unwrap();
        let kr_calendar: KrMarketCalendarResponse = kr_calendar(&client).await.unwrap();
        let us_calendar: UsMarketCalendarResponse = us_calendar(&client).await.unwrap();

        assert_eq!(exchange_rate.rate, "1380.5");
        assert_eq!(exchange_rate.mid_rate, "1375");
        assert_eq!(exchange_rate.basis_point, "40");
        assert_eq!(exchange_rate.rate_change_type.0, "UP");
        assert_eq!(kr_calendar.today.date, "2026-03-25");
        assert_eq!(us_calendar.today.date, "2026-03-25");
    }
}
