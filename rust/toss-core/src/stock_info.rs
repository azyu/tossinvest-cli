use serde_json::Value;

use crate::Result;
use crate::client::TossClient;
use crate::models::stock_info::{StockInfo, StockWarning};
use crate::transport::Transport;

pub async fn stocks_json<T: Transport>(client: &TossClient<T>, symbols: &str) -> Result<Value> {
    client
        .get_json(
            "/api/v1/stocks",
            vec![("symbols".to_string(), symbols.to_string())],
            false,
        )
        .await
}

pub async fn warnings_json<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client
        .get_json(
            &format!("/api/v1/stocks/{symbol}/warnings"),
            Vec::new(),
            false,
        )
        .await
}

pub async fn stocks<T: Transport>(client: &TossClient<T>, symbols: &str) -> Result<Vec<StockInfo>> {
    client
        .get_typed(
            "/api/v1/stocks",
            vec![("symbols".to_string(), symbols.to_string())],
            false,
        )
        .await
}

pub async fn warnings<T: Transport>(
    client: &TossClient<T>,
    symbol: &str,
) -> Result<Vec<StockWarning>> {
    client
        .get_typed(
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
    use serde_json::json;

    use super::{stocks, stocks_json, warnings, warnings_json};
    use crate::auth::TokenManager;
    use crate::client::TossClient;
    use crate::config::AppConfig;
    use crate::models::stock_info::{StockInfo, StockWarning};
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
                body: br#"{"result":[]}"#.to_vec(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"result":[]}"#.to_vec(),
            },
        ]));
        let client = client(requests.clone(), responses);

        stocks_json(&client, "AAPL,MSFT").await.unwrap();
        warnings_json(&client, "AAPL").await.unwrap();

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

    #[tokio::test]
    async fn deserializes_unknown_stock_market_and_currency() {
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
                    "result": [
                        {
                            "symbol": "AAPL",
                            "name": "Apple",
                            "englishName": "APPLE INC",
                            "isinCode": "US0378331005",
                            "market": "MYSTERY_MARKET",
                            "securityType": "STOCK",
                            "isCommonShare": true,
                            "status": "ACTIVE",
                            "currency": "XCU",
                            "listDate": "1980-12-12",
                            "delistDate": null,
                            "sharesOutstanding": "1000000000",
                            "leverageFactor": null,
                            "koreanMarketDetail": null
                        }
                    ]
                })
                .to_string()
                .into_bytes(),
            },
            HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: json!({
                    "result": [
                        {
                            "warningType": "UNKNOWN_WARNING",
                            "exchange": "KRX",
                            "startDate": "2026-03-20",
                            "endDate": null
                        }
                    ]
                })
                .to_string()
                .into_bytes(),
            },
        ]));
        let client = client(requests, responses);

        let stocks: Vec<StockInfo> = stocks(&client, "AAPL").await.unwrap();
        let warnings: Vec<StockWarning> = warnings(&client, "AAPL").await.unwrap();

        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].market.0, "MYSTERY_MARKET");
        assert_eq!(stocks[0].currency.0, "XCU");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].warning_type.0, "UNKNOWN_WARNING");
    }
    #[test]
    fn deserializes_omitted_or_null_nxt_trading_suspended() {
        let omitted = serde_json::json!({
            "symbol": "AAPL",
            "name": "Apple",
            "englishName": "APPLE INC",
            "isinCode": "US0378331005",
            "market": "NASDAQ",
            "securityType": "STOCK",
            "isCommonShare": true,
            "status": "ACTIVE",
            "currency": "USD",
            "listDate": "1980-12-12",
            "delistDate": null,
            "sharesOutstanding": "1000000000",
            "leverageFactor": null,
            "koreanMarketDetail": {
                "liquidationTrading": false,
                "nxtSupported": true,
                "krxTradingSuspended": false
            }
        });
        let with_null = serde_json::json!({
            "symbol": "AAPL",
            "name": "Apple",
            "englishName": "APPLE INC",
            "isinCode": "US0378331005",
            "market": "NASDAQ",
            "securityType": "STOCK",
            "isCommonShare": true,
            "status": "ACTIVE",
            "currency": "USD",
            "listDate": "1980-12-12",
            "delistDate": null,
            "sharesOutstanding": "1000000000",
            "leverageFactor": null,
            "koreanMarketDetail": {
                "liquidationTrading": false,
                "nxtSupported": true,
                "krxTradingSuspended": false,
                "nxtTradingSuspended": null
            }
        });

        let omitted: StockInfo = serde_json::from_value(omitted).unwrap();
        let with_null: StockInfo = serde_json::from_value(with_null).unwrap();

        assert_eq!(omitted.symbol, "AAPL");
        assert_eq!(with_null.symbol, "AAPL");
    }
    #[test]
    fn serializes_omitted_optional_stock_fields_without_nulls() {
        let stock = StockInfo {
            symbol: "AAPL".to_string(),
            name: "Apple".to_string(),
            english_name: "APPLE INC".to_string(),
            isin_code: "US0378331005".to_string(),
            market: crate::models::stock_info::StockMarket("NASDAQ".to_string()),
            security_type: crate::models::stock_info::SecurityType("STOCK".to_string()),
            is_common_share: true,
            status: crate::models::stock_info::StockStatus("ACTIVE".to_string()),
            currency: crate::models::common::Currency("USD".to_string()),
            list_date: None,
            delist_date: None,
            shares_outstanding: "1000000000".to_string(),
            leverage_factor: None,
            korean_market_detail: None,
        };

        let value = serde_json::to_value(stock).unwrap();
        assert!(value.get("listDate").is_none(), "{value}");
        assert!(value.get("delistDate").is_none(), "{value}");
        assert!(value.get("leverageFactor").is_none(), "{value}");
        assert!(value.get("koreanMarketDetail").is_none(), "{value}");
    }
}
