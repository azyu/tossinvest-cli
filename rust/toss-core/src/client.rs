use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::auth::TokenManager;
use crate::config::AppConfig;
use crate::error::{Result, TossError};
use crate::transport::{Header, HttpMethod, HttpRequest, ReqwestTransport, Transport};

#[derive(Debug, Clone)]
pub struct TossClient<T: Transport> {
    config: AppConfig,
    token_manager: TokenManager<T>,
    transport: T,
}

#[derive(Debug, Deserialize)]
struct SuccessEnvelope {
    result: Value,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    code: Option<String>,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

impl TossClient<ReqwestTransport> {
    pub fn new(config: AppConfig) -> Result<Self> {
        let transport = ReqwestTransport::new("https://openapi.tossinvest.com")?;
        let token_manager = TokenManager::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            transport.clone(),
        )?;
        Ok(Self::new_with_parts(config, token_manager, transport))
    }
}

impl<T: Transport> TossClient<T> {
    pub fn new_with_parts(config: AppConfig, token_manager: TokenManager<T>, transport: T) -> Self {
        Self {
            config,
            token_manager,
            transport,
        }
    }

    pub async fn check_token(&self) -> Result<()> {
        self.token_manager.get_token().await.map(|_| ())
    }

    pub async fn get_json(
        &self,
        path: &str,
        query: Vec<(String, String)>,
        account_required: bool,
    ) -> Result<Value> {
        let account_seq = if account_required {
            Some(self.config.account_seq.ok_or_else(|| {
                TossError::Validation(
                    "account sequence is required; run `toss account list` then `toss account use <accountSeq>`"
                        .to_string(),
                )
            })?)
        } else {
            None
        };
        let token = self.token_manager.get_token().await?;
        let mut headers = vec![
            Header {
                name: "accept".to_string(),
                value: "application/json".to_string(),
            },
            Header {
                name: "authorization".to_string(),
                value: format!("Bearer {token}"),
            },
        ];
        if let Some(account_seq) = account_seq {
            headers.push(Header {
                name: "X-Tossinvest-Account".to_string(),
                value: account_seq.to_string(),
            });
        }
        let response = self
            .transport
            .send(HttpRequest {
                method: HttpMethod::Get,
                path: path.to_string(),
                query,
                headers,
                body: None,
            })
            .await?;
        parse_response(response.status, &response.headers, &response.body)
    }
    pub async fn get_typed<R>(
        &self,
        path: &str,
        query: Vec<(String, String)>,
        account_required: bool,
    ) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let value = self.get_json(path, query, account_required).await?;
        Ok(serde_json::from_value(value)?)
    }
}

fn parse_response(status: u16, headers: &[Header], body: &[u8]) -> Result<Value> {
    if status == 429 {
        return Err(TossError::RateLimit {
            message: "rate limit exceeded".to_string(),
            retry_after: header_value(headers, "retry-after"),
            request_id: header_value(headers, "x-request-id"),
        });
    }
    if status >= 400 {
        if let Ok(envelope) = serde_json::from_slice::<ErrorEnvelope>(body) {
            return Err(TossError::Api {
                status: Some(status),
                code: envelope.error.code,
                message: envelope.error.message,
                request_id: envelope
                    .error
                    .request_id
                    .or_else(|| header_value(headers, "x-request-id")),
            });
        }
        return Err(TossError::Api {
            status: Some(status),
            code: None,
            message: String::from_utf8_lossy(body).to_string(),
            request_id: header_value(headers, "x-request-id"),
        });
    }
    let envelope: SuccessEnvelope = serde_json::from_slice(body)?;
    Ok(envelope.result)
}

fn header_value(headers: &[Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.clone())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;

    use super::TossClient;
    use crate::auth::TokenManager;
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

    #[tokio::test]
    async fn injects_bearer_and_account_header() {
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
                body: br#"{"result":{"items":[{"symbol":"AAPL"}]}}"#.to_vec(),
            },
        ]));
        let transport = QueueTransport {
            requests: requests.clone(),
            responses,
        };
        let tempdir = tempfile::tempdir().unwrap();
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempdir.path().join("token.json"),
            transport.clone(),
        );
        let client = TossClient::new_with_parts(
            AppConfig {
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                account_seq: Some(77),
            },
            token_manager,
            transport,
        );

        let value = client
            .get_json("/api/v1/holdings", Vec::new(), true)
            .await
            .unwrap();
        assert_eq!(value["items"][0]["symbol"], "AAPL");

        let captured = requests.lock();
        let api_request = &captured[1];
        assert!(
            api_request
                .headers
                .iter()
                .any(|h| h.name == "authorization" && h.value == "Bearer token-1")
        );
        assert!(
            api_request
                .headers
                .iter()
                .any(|h| h.name == "X-Tossinvest-Account" && h.value == "77")
        );
    }

    #[tokio::test]
    async fn requires_account_for_account_bound_call() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = QueueTransport {
            requests,
            responses: Arc::new(Mutex::new(Vec::new())),
        };
        let tempdir = tempfile::tempdir().unwrap();
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempdir.path().join("token.json"),
            transport.clone(),
        );
        let client = TossClient::new_with_parts(
            AppConfig {
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                account_seq: None,
            },
            token_manager,
            transport,
        );
        let err = client
            .get_json("/api/v1/holdings", Vec::new(), true)
            .await
            .unwrap_err();

        assert!(err.to_string().starts_with("validation error:"), "{err}");
        assert!(err.to_string().contains("toss account list"), "{err}");
        assert!(
            err.to_string().contains("toss account use <accountSeq>"),
            "{err}"
        );
    }
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct TypedProbe {
        symbol: String,
        last_price: serde_json::Value,
    }

    #[tokio::test]
    async fn parses_typed_result_without_floating_point() {
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
                body: br#"{"result":{"symbol":"AAPL","last_price":"181.23"}}"#.to_vec(),
            },
        ]));
        let transport = QueueTransport {
            requests: requests.clone(),
            responses,
        };
        let tempdir = tempfile::tempdir().unwrap();
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempdir.path().join("token.json"),
            transport.clone(),
        );
        let client = TossClient::new_with_parts(
            AppConfig {
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                account_seq: None,
            },
            token_manager,
            transport,
        );

        let typed: TypedProbe = client
            .get_typed("/api/v1/probe", Vec::new(), false)
            .await
            .unwrap();
        assert_eq!(typed.symbol, "AAPL");
        assert_eq!(typed.last_price, serde_json::json!("181.23"));
    }
}
