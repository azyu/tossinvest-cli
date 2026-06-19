use std::fmt;
use std::fs;
use std::io::Write;
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

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
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
        if response.status == 429 {
            return Err(TossError::RateLimit {
                message: "rate limit exceeded".to_string(),
                retry_after: header_value(&response.headers, "retry-after"),
                request_id: header_value(&response.headers, "x-request-id"),
            });
        }
        if (response.status == 400 || response.status == 401)
            && serde_json::from_slice::<TokenErrorResponse>(&response.body).is_ok()
        {
            return Err(TossError::Auth(token_error_message(&response.body)));
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
        let _ = write_cached_token(&self.cache_path, &payload);
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

fn token_error_message(body: &[u8]) -> String {
    if let Ok(error) = serde_json::from_slice::<TokenErrorResponse>(body) {
        return error.error_description.unwrap_or(error.error);
    }
    String::from_utf8_lossy(body).to_string()
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
fn write_cached_token(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| std::borrow::Cow::Borrowed("token.json"));
    let pid = std::process::id();

    for attempt in 0..16u32 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_path = dir.join(format!(".{file_name}.{pid}.{nanos}.{attempt}.tmp"));
        let mut file = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temp_path)
        {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        };

        let result = (|| {
            file.write_all(payload)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp_path, path)
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        return result;
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "unable to allocate token cache temp file",
    ))
}

#[cfg(not(unix))]
fn write_cached_token(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    fs::write(path, payload)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use async_trait::async_trait;
    use parking_lot::Mutex;

    use super::TokenManager;
    use crate::error::TossError;
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
    #[cfg(unix)]
    #[tokio::test]
    async fn creates_token_cache_with_owner_only_mode() {
        use std::os::unix::fs::PermissionsExt;

        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests,
            response: HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"token-3","token_type":"Bearer","expires_in":86400}"#
                    .to_vec(),
            },
        };
        let tempdir = tempfile::tempdir().unwrap();
        let cache_path = tempdir.path().join("token.json");
        let manager = TokenManager::new_with_cache_path(
            "client-3".to_string(),
            "secret-3".to_string(),
            cache_path.clone(),
            transport,
        );

        assert_eq!(manager.get_token().await.unwrap(), "token-3");

        let mode = std::fs::metadata(&cache_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn rewrites_existing_token_cache_with_owner_only_mode() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests,
            response: HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"token-4","token_type":"Bearer","expires_in":86400}"#
                    .to_vec(),
            },
        };
        let tempdir = tempfile::tempdir().unwrap();
        let cache_path = tempdir.path().join("token.json");
        fs::write(
            &cache_path,
            r#"{
  "client_id": "client-4",
  "access_token": "stale-token",
  "expired_at": "2020-01-01T00:00:00Z"
}"#,
        )
        .unwrap();
        fs::set_permissions(&cache_path, fs::Permissions::from_mode(0o644)).unwrap();
        let before_ino = fs::metadata(&cache_path).unwrap().ino();
        let manager = TokenManager::new_with_cache_path(
            "client-4".to_string(),
            "secret-4".to_string(),
            cache_path.clone(),
            transport,
        );

        assert_eq!(manager.get_token().await.unwrap(), "token-4");

        let metadata = fs::metadata(&cache_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_ne!(metadata.ino(), before_ino);
        let contents = fs::read_to_string(&cache_path).unwrap();
        assert!(contents.contains("\"access_token\": \"token-4\""), "{contents}");
    }

    #[tokio::test]
    async fn classifies_token_endpoint_oauth_error_as_auth() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 401,
                headers: vec![Header {
                    name: "X-Request-Id".to_string(),
                    value: "req-auth".to_string(),
                }],
                body: br#"{"error":"invalid_client","error_description":"bad credentials"}"#
                    .to_vec(),
            },
        };
        let cache_path = tempfile::tempdir().unwrap().path().join("token.json");
        let manager = TokenManager::new_with_cache_path(
            "client-auth".to_string(),
            "secret-auth".to_string(),
            cache_path,
            transport,
        );

        let err = manager.get_token().await.unwrap_err();
        match err {
            TossError::Auth(message) => {
                assert!(message.contains("bad credentials"), "{message}");
            }
            other => panic!("expected auth error, got {other:?}"),
        }

        let captured = requests.lock();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].path, "/oauth2/token");
    }

    #[tokio::test]
    async fn classifies_token_endpoint_429_as_rate_limit() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 429,
                headers: vec![
                    Header {
                        name: "Retry-After".to_string(),
                        value: "120".to_string(),
                    },
                    Header {
                        name: "X-Request-Id".to_string(),
                        value: "req-429".to_string(),
                    },
                ],
                body: b"slow down".to_vec(),
            },
        };
        let cache_path = tempfile::tempdir().unwrap().path().join("token.json");
        let manager = TokenManager::new_with_cache_path(
            "client-429".to_string(),
            "secret-429".to_string(),
            cache_path,
            transport,
        );

        let err = manager.get_token().await.unwrap_err();
        match err {
            TossError::RateLimit {
                message,
                retry_after,
                request_id,
            } => {
                assert_eq!(message, "rate limit exceeded");
                assert_eq!(retry_after.as_deref(), Some("120"));
                assert_eq!(request_id.as_deref(), Some("req-429"));
            }
            other => panic!("expected rate limit error, got {other:?}"),
        }

        let captured = requests.lock();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].path, "/oauth2/token");
    }

    #[tokio::test]
    async fn classifies_token_endpoint_400_as_auth() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 400,
                headers: vec![Header {
                    name: "X-Request-Id".to_string(),
                    value: "req-400".to_string(),
                }],
                body: br#"{"error":"invalid_grant","error_description":"client secret is invalid"}"#
                    .to_vec(),
            },
        };
        let cache_path = tempfile::tempdir().unwrap().path().join("token.json");
        let manager = TokenManager::new_with_cache_path(
            "client-400".to_string(),
            "secret-400".to_string(),
            cache_path,
            transport,
        );

        let err = manager.get_token().await.unwrap_err();
        match err {
            TossError::Auth(message) => {
                assert_eq!(message, "client secret is invalid");
            }
            other => panic!("expected auth error, got {other:?}"),
        }

        assert_eq!(requests.lock().len(), 1);
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
