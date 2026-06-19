use serde_json::Value;

use crate::Result;
use crate::client::TossClient;
use crate::models::market_data::{
    CandlePageResponse, OrderbookResponse, PriceLimitResponse, PriceResponse, Trade,
};
use crate::transport::Transport;

pub async fn prices_json<T: Transport>(client: &TossClient<T>, symbols: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/prices",
            vec![("symbols".to_string(), symbols.to_string())],
            false,
        )
        .await
}

pub async fn orderbook_json<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/orderbook",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn trades_json<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/trades",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn price_limits_json<T: Transport>(
    client: &TossClient<T>,
    symbol: &str,
) -> Result<Value> {
    client
        .get_json(
            "/api/v1/price-limits",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn candles_json<T: Transport>(
    client: &TossClient<T>,
    query: Vec<(String, String)>,
) -> Result<Value> {
    client.get_json("/api/v1/candles", query, false).await
}

pub async fn prices<T: Transport>(
    client: &TossClient<T>,
    symbols: &str,
) -> Result<Vec<PriceResponse>> {
    client
        .get_typed(
            "/api/v1/prices",
            vec![("symbols".to_string(), symbols.to_string())],
            false,
        )
        .await
}

pub async fn orderbook<T: Transport>(
    client: &TossClient<T>,
    symbol: &str,
) -> Result<OrderbookResponse> {
    client
        .get_typed(
            "/api/v1/orderbook",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn trades<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Vec<Trade>> {
    client
        .get_typed(
            "/api/v1/trades",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn price_limits<T: Transport>(
    client: &TossClient<T>,
    symbol: &str,
) -> Result<PriceLimitResponse> {
    client
        .get_typed(
            "/api/v1/price-limits",
            vec![("symbol".to_string(), symbol.to_string())],
            false,
        )
        .await
}

pub async fn candles<T: Transport>(
    client: &TossClient<T>,
    query: Vec<(String, String)>,
) -> Result<CandlePageResponse> {
    client.get_typed("/api/v1/candles", query, false).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::{
        candles, candles_json, orderbook, orderbook_json, price_limits, price_limits_json, prices,
        prices_json, trades, trades_json,
    };
    use crate::auth::TokenManager;
    use crate::client::TossClient;
    use crate::config::AppConfig;
    use crate::models::common::{Currency, MarketCountry};
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

    fn response(body: &[u8]) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: body.to_vec(),
        }
    }

    fn token_response() -> HttpResponse {
        response(br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#)
    }

    #[tokio::test]
    async fn parses_typed_market_data_results_and_unknown_strings() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            response(br#"{"result":[{"symbol":"AAPL","timestamp":"2026-03-25T22:30:00.456+09:00","lastPrice":"181.23","currency":"USD"}]}"#),
            response(br#"{"result":{"timestamp":"2026-03-25T09:30:00.123+09:00","currency":"KRW","asks":[{"price":"72300","volume":"1200"}],"bids":[{"price":"72000","volume":"5200"}]}}"#),
            response(br#"{"result":[{"price":"72000","volume":"120","timestamp":"2026-03-25T09:30:42.000+09:00","currency":"KRW"},{"price":"71900","volume":"50","timestamp":"2026-03-25T09:30:41.500+09:00","currency":"KRW"}]}"#),
            response(br#"{"result":{"timestamp":"2026-03-25T09:30:00.123+09:00","upperLimitPrice":"93000","lowerLimitPrice":"50400","currency":"KRW"}}"#),
            response(br#"{"result":{"candles":[{"timestamp":"2026-03-25T09:00:00+09:00","openPrice":"71600","highPrice":"72300","lowPrice":"71500","closePrice":"72000","volume":"3521000","currency":"KRW"},{"timestamp":"2026-03-25T09:01:00+09:00","openPrice":"72000","highPrice":"72100","lowPrice":"71900","closePrice":"71950","volume":"120000","currency":"KRW"}],"nextBefore":"2026-03-25T09:00:00+09:00"}}"#),
        ]));
        let client = client(requests.clone(), responses);

        let prices = prices(&client, "AAPL").await.unwrap();
        assert_eq!(prices[0].symbol, "AAPL");
        assert_eq!(prices[0].last_price, json!("181.23"));
        assert_eq!(prices[0].currency.0, "USD");
        assert_eq!(
            prices[0].timestamp.as_deref(),
            Some("2026-03-25T22:30:00.456+09:00")
        );

        let orderbook = orderbook(&client, "AAPL").await.unwrap();
        assert_eq!(orderbook.currency.0, "KRW");
        assert_eq!(orderbook.asks[0].price, json!("72300"));
        assert_eq!(orderbook.bids[0].volume, json!("5200"));

        let trades = trades(&client, "AAPL").await.unwrap();
        assert_eq!(trades[0].price, json!("72000"));
        assert_eq!(trades[0].currency.0, "KRW");

        let limits = price_limits(&client, "AAPL").await.unwrap();
        assert_eq!(limits.upper_limit_price, Some(json!("93000")));
        assert_eq!(limits.currency.0, "KRW");

        let candles = candles(
            &client,
            vec![
                ("symbol".to_string(), "AAPL".to_string()),
                ("interval".to_string(), "1d".to_string()),
            ],
        )
        .await
        .unwrap();
        assert_eq!(candles.candles.len(), 2);
        assert_eq!(candles.candles[0].close_price, json!("72000"));
        assert_eq!(
            candles.next_before.as_deref(),
            Some("2026-03-25T09:00:00+09:00")
        );

        assert_eq!(
            serde_json::from_value::<Currency>(json!("ZZZ")).unwrap().0,
            "ZZZ"
        );
        assert_eq!(
            serde_json::from_value::<MarketCountry>(json!("XX"))
                .unwrap()
                .0,
            "XX"
        );
    }

    #[tokio::test]
    async fn routes_market_data_json_requests() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            token_response(),
            response(br#"{"result":[]}"#),
            response(br#"{"result":{}}"#),
            response(br#"{"result":[]}"#),
            response(br#"{"result":{}}"#),
            response(br#"{"result":{}}"#),
        ]));
        let client = client(requests.clone(), responses);

        prices_json(&client, "AAPL,MSFT").await.unwrap();
        orderbook_json(&client, "AAPL").await.unwrap();
        trades_json(&client, "AAPL").await.unwrap();
        price_limits_json(&client, "AAPL").await.unwrap();
        candles_json(
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
