use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::error::{Result, TossError};
use crate::transport::{Header, HttpMethod, HttpRequest, Transport};

#[derive(Clone)]
pub struct TokenManager<T: Transport> {
    client_id: String,
    client_secret: String,
    cache_path: PathBuf,
    transport: T,
    state: Arc<Mutex<TokenState>>,
}

impl<T: Transport> fmt::Debug for TokenManager<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenManager")
            .field("client_id", &self.client_id)
            .field("client_secret", &"[redacted]")
            .field("cache_path", &self.cache_path)
            .field("transport", &"<hidden>")
            .field("state", &"<hidden>")
            .finish()
    }
}

#[derive(Debug, Default)]
struct TokenState {
    token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedToken {
    client_id: String,
    access_token: String,
    expired_at: String,
}

impl<T: Transport> TokenManager<T> {
    pub fn new(client_id: String, client_secret: String, transport: T) -> Result<Self> {
        Ok(Self::new_with_cache_path(
            client_id,
            client_secret,
            default_cache_path()?,
            transport,
        ))
    }

    pub fn new_with_cache_path(
        client_id: String,
        client_secret: String,
        cache_path: PathBuf,
        transport: T,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            cache_path,
            transport,
            state: Arc::new(Mutex::new(TokenState::default())),
        }
    }

    pub async fn get_token(&self) -> Result<String> {
        let mut state = self.state.lock().await;
        if let (Some(token), Some(expires_at)) = (&state.token, state.expires_at)
            && Utc::now() < expires_at - chrono::Duration::minutes(5)
        {
            return Ok(token.clone());
        }
        if let Some((token, expires_at)) = self.load_cached_token() {
            state.token = Some(token.clone());
            state.expires_at = Some(expires_at);
            return Ok(token);
        }

        let (token, expires_at) = self.fetch_token().await?;
        state.token = Some(token.clone());
        state.expires_at = Some(expires_at);
        self.save_cached_token(&token, expires_at);
        Ok(token)
    }

    async fn fetch_token(&self) -> Result<(String, DateTime<Utc>)> {
        let body = form_urlencoded_body(&self.client_id, &self.client_secret);
        let response = self
            .transport
            .send(HttpRequest {
                method: HttpMethod::Post,
                path: "/oauth2/token".to_string(),
                query: Vec::new(),
                headers: vec![Header {
                    name: "content-type".to_string(),
                    value: "application/x-www-form-urlencoded".to_string(),
                }],
                body: Some(body.into_bytes()),
            })
            .await?;
        if response.status == 401 || response.status == 400 {
            return Err(TossError::Auth(
                String::from_utf8_lossy(&response.body).to_string(),
            ));
        }
        if response.status >= 400 {
            return Err(TossError::Api {
                status: Some(response.status),
                code: None,
                message: String::from_utf8_lossy(&response.body).to_string(),
                request_id: header_value(&response.headers, "x-request-id"),
            });
        }
        let payload: TokenResponse = serde_json::from_slice(&response.body)?;
        if !payload.token_type.eq_ignore_ascii_case("bearer") {
            return Err(TossError::Auth(format!(
                "unsupported token type: {}",
                payload.token_type
            )));
        }
        let expires_at = Utc::now() + chrono::Duration::seconds(payload.expires_in);
        Ok((payload.access_token, expires_at))
    }

    fn load_cached_token(&self) -> Option<(String, DateTime<Utc>)> {
        let data = fs::read_to_string(&self.cache_path).ok()?;
        let cached: CachedToken = serde_json::from_str(&data).ok()?;
        if cached.client_id != self.client_id {
            return None;
        }
        let expires_at = DateTime::parse_from_rfc3339(&cached.expired_at)
            .ok()?
            .with_timezone(&Utc);
        if Utc::now() >= expires_at - chrono::Duration::minutes(5) {
            return None;
        }
        Some((cached.access_token, expires_at))
    }

    fn save_cached_token(&self, token: &str, expires_at: DateTime<Utc>) {
        if let Some(parent) = self.cache_path.parent()
            && fs::create_dir_all(parent).is_err()
        {
            return;
        }
        let cached = CachedToken {
            client_id: self.client_id.clone(),
            access_token: token.to_string(),
            expired_at: expires_at.to_rfc3339(),
        };
        let Ok(payload) = serde_json::to_vec_pretty(&cached) else {
            return;
        };
        if fs::write(&self.cache_path, payload).is_ok() {
            let _ = set_file_mode(&self.cache_path);
        }
    }
}

fn default_cache_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| TossError::Config("determining home directory".to_string()))?;
    Ok(home.join(".tossinvest").join("token.json"))
}

fn header_value(headers: &[Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.clone())
}

fn form_urlencoded_body(client_id: &str, client_secret: &str) -> String {
    let mut body = String::with_capacity(
        "grant_type=client_credentials&client_id=&client_secret=".len()
            + client_id.len() * 3
            + client_secret.len() * 3,
    );
    body.push_str("grant_type=client_credentials&client_id=");
    push_form_encoded(&mut body, client_id);
    body.push_str("&client_secret=");
    push_form_encoded(&mut body, client_secret);
    body
}

fn push_form_encoded(out: &mut String, value: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 0x0f) as usize] as char);
            }
        }
    }
}

#[cfg(unix)]
fn set_file_mode(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_file_mode(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;

    use super::TokenManager;
    use crate::transport::{Header, HttpMethod, HttpRequest, HttpResponse, Transport};

    #[derive(Clone, Debug)]
    struct RecordingTransport {
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        response: HttpResponse,
    }

    #[async_trait]
    impl Transport for RecordingTransport {
        async fn send(&self, request: HttpRequest) -> crate::Result<HttpResponse> {
            self.requests.lock().push(request);
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn encodes_form_delimiters_and_whitespace_in_credentials() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#
                    .to_vec(),
            },
        };
        let cache_path = tempfile::tempdir().unwrap().path().join("token.json");
        let manager = TokenManager::new_with_cache_path(
            "client id &=".to_string(),
            "sec ret+two=3&4".to_string(),
            cache_path,
            transport,
        );

        let token = manager.get_token().await.unwrap();
        assert_eq!(token, "token-1");

        let captured = requests.lock();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].body.as_deref(), Some(b"grant_type=client_credentials&client_id=client+id+%26%3D&client_secret=sec+ret%2Btwo%3D3%264".as_slice()));
    }

    #[test]
    fn debug_redacts_client_secret() {
        let manager = TokenManager::new_with_cache_path(
            "client-9".to_string(),
            "super-secret".to_string(),
            tempfile::tempdir().unwrap().path().join("token.json"),
            RecordingTransport {
                requests: Arc::new(Mutex::new(Vec::new())),
                response: HttpResponse {
                    status: 200,
                    headers: Vec::new(),
                    body: Vec::new(),
                },
            },
        );

        let debug = format!("{manager:?}");

        assert!(!debug.contains("super-secret"), "{debug}");
        assert!(debug.contains("client_secret: \"[redacted]\""), "{debug}");
    }

    #[tokio::test]
    async fn posts_form_urlencoded_token_request() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#
                    .to_vec(),
            },
        };
        let cache_path = tempfile::tempdir().unwrap().path().join("token.json");
        let manager = TokenManager::new_with_cache_path(
            "client-1".to_string(),
            "secret-1".to_string(),
            cache_path,
            transport,
        );

        let token = manager.get_token().await.unwrap();
        assert_eq!(token, "token-1");

        let captured = requests.lock();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].method, HttpMethod::Post);
        assert_eq!(captured[0].path, "/oauth2/token");
        assert!(captured[0].headers.iter().any(|h| h.name == "content-type" && h.value == "application/x-www-form-urlencoded"));
        assert_eq!(
            captured[0].body.as_deref(),
            Some(
                b"grant_type=client_credentials&client_id=client-1&client_secret=secret-1"
                    .as_slice()
            )
        );
    }

    #[tokio::test]
    async fn reuses_cached_token_in_memory() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 200,
                headers: vec![Header {
                    name: "x-test".to_string(),
                    value: "ok".to_string(),
                }],
                body: br#"{"access_token":"token-2","token_type":"Bearer","expires_in":86400}"#
                    .to_vec(),
            },
        };
        let cache_path = tempfile::tempdir().unwrap().path().join("token.json");
        let manager = TokenManager::new_with_cache_path(
            "client-2".to_string(),
            "secret-2".to_string(),
            cache_path,
            transport,
        );

        assert_eq!(manager.get_token().await.unwrap(), "token-2");
        assert_eq!(manager.get_token().await.unwrap(), "token-2");
        assert_eq!(requests.lock().len(), 1);
    }
    #[tokio::test]
    async fn does_not_reuse_cached_token_for_different_client_id() {
        let first_requests = Arc::new(Mutex::new(Vec::new()));
        let second_requests = Arc::new(Mutex::new(Vec::new()));
        let tempdir = tempfile::tempdir().unwrap();
        let cache_path = tempdir.path().join("token.json");

        let first_manager = TokenManager::new_with_cache_path(
            "client-a".to_string(),
            "secret-a".to_string(),
            cache_path.clone(),
            RecordingTransport {
                requests: first_requests.clone(),
                response: HttpResponse {
                    status: 200,
                    headers: Vec::new(),
                    body: br#"{"access_token":"token-a","token_type":"Bearer","expires_in":86400}"#
                        .to_vec(),
                },
            },
        );
        assert_eq!(first_manager.get_token().await.unwrap(), "token-a");
        assert_eq!(first_requests.lock().len(), 1);

        let cached: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cache_path).unwrap()).unwrap();
        assert_eq!(cached["client_id"], "client-a");
        assert!(cached.get("client_secret").is_none(), "{cached}");

        let second_manager = TokenManager::new_with_cache_path(
            "client-b".to_string(),
            "secret-b".to_string(),
            cache_path,
            RecordingTransport {
                requests: second_requests.clone(),
                response: HttpResponse {
                    status: 200,
                    headers: Vec::new(),
                    body: br#"{"access_token":"token-b","token_type":"Bearer","expires_in":86400}"#
                        .to_vec(),
                },
            },
        );

        assert_eq!(second_manager.get_token().await.unwrap(), "token-b");
        assert_eq!(second_requests.lock().len(), 1);
    }
}
