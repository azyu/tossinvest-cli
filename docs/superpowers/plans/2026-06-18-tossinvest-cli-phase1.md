# Tossinvest CLI Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Phase 1 read-only Toss Securities Open API CLI with config, auth token plumbing, read-only market/account commands, stable JSON envelopes, and tested request construction.

**Architecture:** Create a Rust workspace with `toss-core` for config/auth/client/endpoint wrappers and `toss-cli` for clap parsing, command dispatch, and rendering. Keep request construction testable through an in-memory transport trait so tests do not require real Toss credentials. Defer typed-model completeness and mutating order commands to later phases.

**Tech Stack:** Rust edition 2024, `clap`, `tokio`, `reqwest` with `rustls-tls`, `serde`, `serde_json`, `serde_yaml`, `thiserror`, `anyhow`, `dirs`, `async-trait`, `tempfile`, `unicode-width`.

## Global Constraints

- Use the OpenAPI document at `https://openapi.tossinvest.com/openapi-docs/latest/openapi.json` as the source of truth.
- Do not copy KIS-specific TR-ID, hashkey, virtual/real environment, or WebSocket concepts.
- Default config path is `~/.config/tossinvest/config.yaml`.
- Environment overrides are `TOSSINVEST_CLIENT_ID`, `TOSSINVEST_CLIENT_SECRET`, and `TOSSINVEST_ACCOUNT_SEQ`.
- Token cache path is `~/.tossinvest/token.json`.
- Never print `client_secret` or access tokens in normal command output.
- JSON success envelope is `{ "ok": true, "command": "...", "data": ... }`.
- JSON error envelope is `{ "ok": false, "command": "...", "error": { "kind": "...", "code": null, "message": "...", "requestId": null } }`.
- Phase 1 excludes create/modify/cancel order calls.
- Prices, quantities, and money values remain strings or `serde_json::Value`; do not use floating-point types.

---

## File Structure

Create:

- `rust/Cargo.toml` — workspace root.
- `rust/toss-core/Cargo.toml` — core crate manifest.
- `rust/toss-core/src/lib.rs` — public module exports.
- `rust/toss-core/src/error.rs` — shared error enum and result alias.
- `rust/toss-core/src/config.rs` — file/env config loading and account persistence.
- `rust/toss-core/src/transport.rs` — request/response structs, `Transport` trait, `ReqwestTransport`.
- `rust/toss-core/src/auth.rs` — OAuth2 token manager and cache.
- `rust/toss-core/src/client.rs` — authenticated API client, account header injection, response envelope parsing.
- `rust/toss-core/src/market_data.rs` — prices, orderbook, trades, limits, candles wrappers.
- `rust/toss-core/src/stock_info.rs` — stocks and warnings wrappers.
- `rust/toss-core/src/market_info.rs` — exchange-rate and calendars wrappers.
- `rust/toss-core/src/account.rs` — account list wrapper.
- `rust/toss-core/src/asset.rs` — holdings wrapper.
- `rust/toss-cli/Cargo.toml` — CLI crate manifest.
- `rust/toss-cli/src/main.rs` — binary entrypoint.
- `rust/toss-cli/src/lib.rs` — CLI library exports for tests.
- `rust/toss-cli/src/cli.rs` — clap command definitions.
- `rust/toss-cli/src/runtime.rs` — dispatch and output envelopes.
- `rust/toss-cli/src/render.rs` — compact table/key-value text output helpers.
- `rust/toss-cli/tests/cli_smoke.rs` — binary and parser smoke tests.
- `README.md` — Phase 1 usage, config, and safety notes.

---

### Task 1: Workspace and Config Foundation

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/toss-core/Cargo.toml`
- Create: `rust/toss-core/src/lib.rs`
- Create: `rust/toss-core/src/error.rs`
- Create: `rust/toss-core/src/config.rs`

**Interfaces:**
- Produces: `toss_core::config::AppConfig`
- Produces: `toss_core::config::load(config_path: Option<&Path>, account_override: Option<&str>) -> toss_core::Result<AppConfig>`
- Produces: `toss_core::config::save_account_seq(config_path: Option<&Path>, account_seq: u64) -> toss_core::Result<PathBuf>`
- Produces: `toss_core::error::{TossError, Result}`

- [ ] **Step 1: Write config tests**

Create `rust/toss-core/src/config.rs` with the tests first at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use std::fs;

    use super::{load, read_file_config, save_account_seq};

    #[test]
    fn allows_missing_default_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.yaml");
        let file = read_file_config(&path, true).unwrap();
        assert!(file.client_id.is_none());
        assert!(file.account_seq.is_none());
    }

    #[test]
    fn rejects_missing_explicit_config_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.yaml");
        let err = load(Some(&path), None).unwrap_err();
        assert!(err.to_string().contains("No such file"));
    }

    #[test]
    fn loads_yaml_file_and_account_override() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(
            &path,
            r#"
client_id: "client-file"
client_secret: "secret-file"
account_seq: 7
"#,
        )
        .unwrap();

        let config = load(Some(&path), Some("9")).unwrap();
        assert_eq!(config.client_id, "client-file");
        assert_eq!(config.client_secret, "secret-file");
        assert_eq!(config.account_seq, Some(9));
    }

    #[test]
    fn saves_account_seq_without_losing_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(
            &path,
            r#"client_id: "client-file"
client_secret: "secret-file"
"#,
        )
        .unwrap();

        let saved = save_account_seq(Some(&path), 42).unwrap();
        assert_eq!(saved, path);
        let contents = fs::read_to_string(saved).unwrap();
        assert!(contents.contains("client_id: client-file"));
        assert!(contents.contains("client_secret: secret-file"));
        assert!(contents.contains("account_seq: 42"));
    }
}
```

- [ ] **Step 2: Run config tests to verify they fail**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core config::tests
```

Expected: fail because the workspace and config implementation do not exist yet.

- [ ] **Step 3: Create manifests**

Create `rust/Cargo.toml`:

```toml
[workspace]
members = ["toss-core", "toss-cli"]
resolver = "2"
```

Create `rust/toss-core/Cargo.toml`:

```toml
[package]
name = "toss-core"
version = "0.1.0"
edition = "2024"

[dependencies]
async-trait = "0.1.88"
chrono = { version = "0.4.40", features = ["clock"] }
dirs = "6.0.0"
reqwest = { version = "0.12.14", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
serde_yaml = "0.9.34"
thiserror = "2.0.12"
tokio = { version = "1.44.1", features = ["sync", "time"] }

[dev-dependencies]
tempfile = "3.18.0"
tokio = { version = "1.44.1", features = ["macros", "rt-multi-thread"] }
```

Create `rust/toss-core/src/lib.rs`:

```rust
pub mod config;
pub mod error;

pub use error::{Result, TossError};
```

Create `rust/toss-core/src/error.rs`:

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, TossError>;

#[derive(Debug, Error)]
pub enum TossError {
    #[error("config error: {0}")]
    Config(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("api error: {message}")]
    Api {
        status: Option<u16>,
        code: Option<String>,
        message: String,
        request_id: Option<String>,
    },
    #[error("rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<String>,
        request_id: Option<String>,
    },
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}
```

- [ ] **Step 4: Implement config**

Replace `rust/toss-core/src/config.rs` above the tests with:

```rust
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, TossError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub client_id: String,
    pub client_secret: String,
    pub account_seq: Option<u64>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct FileConfig {
    client_id: Option<String>,
    client_secret: Option<String>,
    account_seq: Option<u64>,
}

pub fn load(config_path: Option<&Path>, account_override: Option<&str>) -> Result<AppConfig> {
    let path = default_config_path(config_path)?;
    let file = read_file_config(&path, config_path.is_none())?;

    let client_id = env::var("TOSSINVEST_CLIENT_ID")
        .ok()
        .or(file.client_id)
        .unwrap_or_default();
    let client_secret = env::var("TOSSINVEST_CLIENT_SECRET")
        .ok()
        .or(file.client_secret)
        .unwrap_or_default();
    let account_seq = account_override
        .map(parse_account_seq)
        .transpose()?
        .or_else(|| env::var("TOSSINVEST_ACCOUNT_SEQ").ok().map(|raw| parse_account_seq(&raw)))
        .transpose()?
        .or(file.account_seq);

    Ok(AppConfig {
        client_id,
        client_secret,
        account_seq,
    })
}

pub fn save_account_seq(config_path: Option<&Path>, account_seq: u64) -> Result<PathBuf> {
    let path = default_config_path(config_path)?;
    let mut file = read_file_config(&path, true)?;
    file.account_seq = Some(account_seq);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_yaml::to_string(&file)?;
    fs::write(&path, payload)?;
    Ok(path)
}

fn parse_account_seq(raw: &str) -> Result<u64> {
    raw.parse::<u64>()
        .map_err(|_| TossError::Config(format!("invalid account sequence: {raw}")))
}

fn read_file_config(path: &Path, allow_missing: bool) -> Result<FileConfig> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(serde_yaml::from_str(&contents)?),
        Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {
            Ok(FileConfig::default())
        }
        Err(error) => Err(error.into()),
    }
}

fn default_config_path(config_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = config_path {
        return Ok(path.to_path_buf());
    }
    let home = dirs::home_dir()
        .ok_or_else(|| TossError::Config("determining home directory".to_string()))?;
    Ok(home.join(".config").join("tossinvest").join("config.yaml"))
}
```

- [ ] **Step 5: Run config tests to verify they pass**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core config::tests
```

Expected: all config tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/Cargo.toml rust/toss-core/Cargo.toml rust/toss-core/src/lib.rs rust/toss-core/src/error.rs rust/toss-core/src/config.rs
git commit -m "feat: add toss config foundation"
```

---

### Task 2: Transport and Auth Token Manager

**Files:**
- Modify: `rust/toss-core/src/lib.rs`
- Create: `rust/toss-core/src/transport.rs`
- Create: `rust/toss-core/src/auth.rs`

**Interfaces:**
- Consumes: `toss_core::config::AppConfig`
- Produces: `toss_core::transport::{Header, HttpMethod, HttpRequest, HttpResponse, Transport, ReqwestTransport}`
- Produces: `toss_core::auth::TokenManager<T: Transport>`
- Produces: `TokenManager::get_token(&self) -> impl Future<Output = Result<String>>`

- [ ] **Step 1: Write auth tests**

Create `rust/toss-core/src/transport.rs` with interface scaffolding and a test transport helper:

```rust
use async_trait::async_trait;

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<Header>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

#[async_trait]
pub trait Transport: Clone + Send + Sync + 'static {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse>;
}
```

Create `rust/toss-core/src/auth.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::TokenManager;
    use crate::transport::{Header, HttpMethod, HttpRequest, HttpResponse, Transport};

    #[derive(Clone)]
    struct RecordingTransport {
        requests: Arc<Mutex<Vec<HttpRequest>>>,
        response: HttpResponse,
    }

    #[async_trait]
    impl Transport for RecordingTransport {
        async fn send(&self, request: HttpRequest) -> crate::Result<HttpResponse> {
            self.requests.lock().unwrap().push(request);
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn posts_form_urlencoded_token_request() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#.to_vec(),
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

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].method, HttpMethod::Post);
        assert_eq!(captured[0].path, "/oauth2/token");
        assert!(captured[0].headers.iter().any(|h| h.name == "content-type" && h.value == "application/x-www-form-urlencoded"));
        assert_eq!(captured[0].body.as_deref(), Some(b"grant_type=client_credentials&client_id=client-1&client_secret=secret-1".as_slice()));
    }

    #[tokio::test]
    async fn reuses_cached_token_in_memory() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = RecordingTransport {
            requests: requests.clone(),
            response: HttpResponse {
                status: 200,
                headers: vec![Header { name: "x-test".to_string(), value: "ok".to_string() }],
                body: br#"{"access_token":"token-2","token_type":"Bearer","expires_in":86400}"#.to_vec(),
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
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
}
```

- [ ] **Step 2: Run auth tests to verify they fail**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core auth::tests
```

Expected: fail because `auth` is not exported and `TokenManager` is not implemented.

- [ ] **Step 3: Export modules and implement reqwest transport**

Modify `rust/toss-core/src/lib.rs`:

```rust
pub mod auth;
pub mod config;
pub mod error;
pub mod transport;

pub use error::{Result, TossError};
```

Append to `rust/toss-core/src/transport.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    base_url: String,
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            base_url: base_url.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
        })
    }
}

#[async_trait]
impl Transport for ReqwestTransport {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        let url = format!("{}{}", self.base_url, request.path);
        let mut builder = match request.method {
            HttpMethod::Get => self.client.get(url).query(&request.query),
            HttpMethod::Post => self.client.post(url).query(&request.query),
        };
        for header in request.headers {
            builder = builder.header(header.name, header.value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let response = builder.send().await?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|value| Header {
                    name: name.as_str().to_string(),
                    value: value.to_string(),
                })
            })
            .collect();
        let body = response.bytes().await?.to_vec();
        Ok(HttpResponse { status, headers, body })
    }
}
```

- [ ] **Step 4: Implement token manager**

Create the implementation in `rust/toss-core/src/auth.rs` above the tests:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::error::{Result, TossError};
use crate::transport::{Header, HttpMethod, HttpRequest, Transport};

#[derive(Debug, Clone)]
pub struct TokenManager<T: Transport> {
    client_id: String,
    client_secret: String,
    cache_path: PathBuf,
    transport: T,
    state: std::sync::Arc<Mutex<TokenState>>,
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
            state: std::sync::Arc::new(Mutex::new(TokenState::default())),
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
        let body = format!(
            "grant_type=client_credentials&client_id={}&client_secret={}",
            self.client_id, self.client_secret
        );
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
            return Err(TossError::Auth(String::from_utf8_lossy(&response.body).to_string()));
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
            return Err(TossError::Auth(format!("unsupported token type: {}", payload.token_type)));
        }
        let expires_at = Utc::now() + chrono::Duration::seconds(payload.expires_in);
        Ok((payload.access_token, expires_at))
    }

    fn load_cached_token(&self) -> Option<(String, DateTime<Utc>)> {
        let data = fs::read_to_string(&self.cache_path).ok()?;
        let cached: CachedToken = serde_json::from_str(&data).ok()?;
        let expires_at = DateTime::parse_from_rfc3339(&cached.expired_at).ok()?.with_timezone(&Utc);
        if Utc::now() >= expires_at - chrono::Duration::minutes(5) {
            return None;
        }
        Some((cached.access_token, expires_at))
    }

    fn save_cached_token(&self, token: &str, expires_at: DateTime<Utc>) {
        if let Some(parent) = self.cache_path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        let cached = CachedToken {
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
```

- [ ] **Step 5: Run auth tests to verify they pass**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core auth::tests
```

Expected: auth tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-core/src/lib.rs rust/toss-core/src/transport.rs rust/toss-core/src/auth.rs
git commit -m "feat: add toss auth transport"
```

---

### Task 3: Authenticated Client and Endpoint Wrappers

**Files:**
- Modify: `rust/toss-core/src/lib.rs`
- Create: `rust/toss-core/src/client.rs`
- Create: `rust/toss-core/src/market_data.rs`
- Create: `rust/toss-core/src/stock_info.rs`
- Create: `rust/toss-core/src/market_info.rs`
- Create: `rust/toss-core/src/account.rs`
- Create: `rust/toss-core/src/asset.rs`

**Interfaces:**
- Consumes: `TokenManager<T>` and `Transport`
- Produces: `TossClient<T>::get_json(&self, path: &str, query: Vec<(String, String)>, account_required: bool) -> Result<Value>`
- Produces endpoint functions returning `serde_json::Value`

- [ ] **Step 1: Write client tests**

Create `rust/toss-core/src/client.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

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
            self.requests.lock().unwrap().push(request);
            Ok(self.responses.lock().unwrap().remove(0))
        }
    }

    #[tokio::test]
    async fn injects_bearer_and_account_header() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(vec![
            HttpResponse { status: 200, headers: Vec::new(), body: br#"{"access_token":"token-1","token_type":"Bearer","expires_in":86400}"#.to_vec() },
            HttpResponse { status: 200, headers: Vec::new(), body: br#"{"result":{"items":[{"symbol":"AAPL"}]}}"#.to_vec() },
        ]));
        let transport = QueueTransport { requests: requests.clone(), responses };
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempfile::tempdir().unwrap().path().join("token.json"),
            transport.clone(),
        );
        let client = TossClient::new_with_parts(
            AppConfig { client_id: "client".to_string(), client_secret: "secret".to_string(), account_seq: Some(77) },
            token_manager,
            transport,
        );

        let value = client.get_json("/api/v1/holdings", Vec::new(), true).await.unwrap();
        assert_eq!(value["items"][0]["symbol"], "AAPL");

        let captured = requests.lock().unwrap();
        let api_request = &captured[1];
        assert!(api_request.headers.iter().any(|h| h.name == "authorization" && h.value == "Bearer token-1"));
        assert!(api_request.headers.iter().any(|h| h.name == "X-Tossinvest-Account" && h.value == "77"));
    }

    #[tokio::test]
    async fn requires_account_for_account_bound_call() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = QueueTransport { requests, responses: Arc::new(Mutex::new(Vec::new())) };
        let token_manager = TokenManager::new_with_cache_path(
            "client".to_string(),
            "secret".to_string(),
            tempfile::tempdir().unwrap().path().join("token.json"),
            transport.clone(),
        );
        let client = TossClient::new_with_parts(
            AppConfig { client_id: "client".to_string(), client_secret: "secret".to_string(), account_seq: None },
            token_manager,
            transport,
        );

        let err = client.get_json("/api/v1/holdings", Vec::new(), true).await.unwrap_err();
        assert!(err.to_string().contains("account sequence is required"));
    }
}
```

- [ ] **Step 2: Run client tests to verify they fail**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core client::tests
```

Expected: fail because `TossClient` is not implemented.

- [ ] **Step 3: Implement client**

Create `rust/toss-core/src/client.rs` above the tests:

```rust
use serde::Deserialize;
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
        Self { config, token_manager, transport }
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
        let token = self.token_manager.get_token().await?;
        let mut headers = vec![
            Header { name: "accept".to_string(), value: "application/json".to_string() },
            Header { name: "authorization".to_string(), value: format!("Bearer {token}") },
        ];
        if account_required {
            let account_seq = self.config.account_seq.ok_or_else(|| {
                TossError::Config("account sequence is required; run `toss account list` then `toss account use <accountSeq>`".to_string())
            })?;
            headers.push(Header { name: "X-Tossinvest-Account".to_string(), value: account_seq.to_string() });
        }
        let response = self.transport.send(HttpRequest {
            method: HttpMethod::Get,
            path: path.to_string(),
            query,
            headers,
            body: None,
        }).await?;
        parse_response(response.status, &response.headers, &response.body)
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
                request_id: envelope.error.request_id.or_else(|| header_value(headers, "x-request-id")),
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
```

- [ ] **Step 4: Add endpoint wrapper modules**

Modify `rust/toss-core/src/lib.rs`:

```rust
pub mod account;
pub mod asset;
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod market_data;
pub mod market_info;
pub mod stock_info;
pub mod transport;

pub use error::{Result, TossError};
```

Create `rust/toss-core/src/market_data.rs`:

```rust
use serde_json::Value;

use crate::client::TossClient;
use crate::transport::Transport;
use crate::Result;

pub async fn prices<T: Transport>(client: &TossClient<T>, symbols: &str) -> Result<Value> {
    client.get_json("/api/v1/prices", vec![("symbols".to_string(), symbols.to_string())], false).await
}

pub async fn orderbook<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client.get_json("/api/v1/orderbook", vec![("symbol".to_string(), symbol.to_string())], false).await
}

pub async fn trades<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client.get_json("/api/v1/trades", vec![("symbol".to_string(), symbol.to_string())], false).await
}

pub async fn price_limits<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client.get_json("/api/v1/price-limits", vec![("symbol".to_string(), symbol.to_string())], false).await
}

pub async fn candles<T: Transport>(client: &TossClient<T>, query: Vec<(String, String)>) -> Result<Value> {
    client.get_json("/api/v1/candles", query, false).await
}
```

Create `rust/toss-core/src/stock_info.rs`:

```rust
use serde_json::Value;

use crate::client::TossClient;
use crate::transport::Transport;
use crate::Result;

pub async fn stocks<T: Transport>(client: &TossClient<T>, symbols: &str) -> Result<Value> {
    client.get_json("/api/v1/stocks", vec![("symbols".to_string(), symbols.to_string())], false).await
}

pub async fn warnings<T: Transport>(client: &TossClient<T>, symbol: &str) -> Result<Value> {
    client.get_json(&format!("/api/v1/stocks/{symbol}/warnings"), Vec::new(), false).await
}
```

Create `rust/toss-core/src/market_info.rs`:

```rust
use serde_json::Value;

use crate::client::TossClient;
use crate::transport::Transport;
use crate::Result;

pub async fn exchange_rate<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/exchange-rate", Vec::new(), false).await
}

pub async fn kr_calendar<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/market-calendar/KR", Vec::new(), false).await
}

pub async fn us_calendar<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/market-calendar/US", Vec::new(), false).await
}
```

Create `rust/toss-core/src/account.rs`:

```rust
use serde_json::Value;

use crate::client::TossClient;
use crate::transport::Transport;
use crate::Result;

pub async fn list<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/accounts", Vec::new(), false).await
}
```

Create `rust/toss-core/src/asset.rs`:

```rust
use serde_json::Value;

use crate::client::TossClient;
use crate::transport::Transport;
use crate::Result;

pub async fn holdings<T: Transport>(client: &TossClient<T>) -> Result<Value> {
    client.get_json("/api/v1/holdings", Vec::new(), true).await
}
```

- [ ] **Step 5: Run core tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-core
```

Expected: all `toss-core` tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-core/src/lib.rs rust/toss-core/src/client.rs rust/toss-core/src/market_data.rs rust/toss-core/src/stock_info.rs rust/toss-core/src/market_info.rs rust/toss-core/src/account.rs rust/toss-core/src/asset.rs
git commit -m "feat: add read only api wrappers"
```

---

### Task 4: CLI Parser and Output Runtime

**Files:**
- Create: `rust/toss-cli/Cargo.toml`
- Create: `rust/toss-cli/src/lib.rs`
- Create: `rust/toss-cli/src/cli.rs`
- Create: `rust/toss-cli/src/render.rs`
- Create: `rust/toss-cli/src/runtime.rs`
- Create: `rust/toss-cli/src/main.rs`
- Create: `rust/toss-cli/tests/cli_smoke.rs`

**Interfaces:**
- Consumes: `toss_core` Phase 1 wrappers
- Produces: binary `toss`
- Produces: `toss_cli::runtime::run(cli: Cli, writer: &mut dyn Write) -> anyhow::Result<()>`
- Produces: `toss_cli::runtime::write_json_error(writer, command, err)`

- [ ] **Step 1: Write CLI parser and binary smoke tests**

Create `rust/toss-cli/tests/cli_smoke.rs`:

```rust
use std::fs;
use std::process::Command;

use clap::Parser;
use toss_cli::cli::{Cli, Command, OutputFormat, QuoteCommand};

#[test]
fn parses_json_price_command() {
    let cli = Cli::parse_from(["toss", "--json", "price", "005930"]);
    assert_eq!(cli.output_format(), OutputFormat::Json);
    match cli.command {
        Command::Price(args) => assert_eq!(args.symbol, "005930"),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_quote_orderbook_command() {
    let cli = Cli::parse_from(["toss", "quote", "orderbook", "AAPL"]);
    match cli.command {
        Command::Quote(args) => match args.command {
            QuoteCommand::Orderbook(symbol) => assert_eq!(symbol.symbol, "AAPL"),
            other => panic!("unexpected quote command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn runs_config_command_through_binary() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    fs::write(&config, "client_id: client-abc\nclient_secret: secret-xyz\naccount_seq: 5\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_toss"))
        .args(["--config", config.to_str().unwrap(), "--json", "config"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"ok\":true"));
    assert!(stdout.contains("\"account_seq\":5"));
    assert!(!stdout.contains("secret-xyz"));
}
```

- [ ] **Step 2: Run CLI tests to verify they fail**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli
```

Expected: fail because `toss-cli` does not exist.

- [ ] **Step 3: Create CLI manifest and parser**

Create `rust/toss-cli/Cargo.toml`:

```toml
[package]
name = "toss-cli"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "toss"
path = "src/main.rs"

[dependencies]
anyhow = "1.0.97"
clap = { version = "4.5.32", features = ["derive"] }
dirs = "6.0.0"
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
tokio = { version = "1.44.1", features = ["macros", "rt-multi-thread"] }
toss-core = { path = "../toss-core" }
unicode-width = "0.2.0"

[dev-dependencies]
tempfile = "3.18.0"
```

Create `rust/toss-cli/src/lib.rs`:

```rust
pub mod cli;
pub mod render;
pub mod runtime;
```

Create `rust/toss-cli/src/cli.rs`:

```rust
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "toss", about = "Toss Securities Open API CLI")]
pub struct Cli {
    #[arg(long, global = true, help = "config file (default: ~/.config/tossinvest/config.yaml)")]
    pub config: Option<PathBuf>,
    #[arg(long, global = true, help = "accountSeq override for account-bound commands")]
    pub account: Option<String>,
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub output: OutputFormat,
    #[arg(long, global = true, help = "print successful command output as JSON")]
    pub json: bool,
    #[arg(long, global = true, help = "suppress extra text in text output")]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn output_format(&self) -> OutputFormat {
        if self.json { OutputFormat::Json } else { self.output }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Config,
    Auth(AuthArgs),
    Price(PriceArgs),
    Quote(QuoteArgs),
    Chart(ChartArgs),
    Stock(StockArgs),
    Market(MarketArgs),
    Account(AccountArgs),
    Holdings,
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Auth(_) => "auth",
            Self::Price(_) => "price",
            Self::Quote(_) => "quote",
            Self::Chart(_) => "chart",
            Self::Stock(_) => "stock",
            Self::Market(_) => "market",
            Self::Account(_) => "account",
            Self::Holdings => "holdings",
        }
    }
}

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Token,
}

#[derive(Debug, Args)]
pub struct PriceArgs {
    pub symbol: String,
    #[arg(long, help = "comma-separated symbols; overrides positional symbol")]
    pub symbols: Option<String>,
}

#[derive(Debug, Args)]
pub struct QuoteArgs {
    #[command(subcommand)]
    pub command: QuoteCommand,
}

#[derive(Debug, Subcommand)]
pub enum QuoteCommand {
    Orderbook(SymbolArg),
    Trades(SymbolArg),
    Limits(SymbolArg),
}

#[derive(Debug, Args)]
pub struct ChartArgs {
    #[command(subcommand)]
    pub command: ChartCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChartCommand {
    Candles(CandlesArgs),
}

#[derive(Debug, Args)]
pub struct CandlesArgs {
    pub symbol: String,
    #[arg(long)]
    pub interval: String,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
}

#[derive(Debug, Args)]
pub struct StockArgs {
    #[command(subcommand)]
    pub command: StockCommand,
}

#[derive(Debug, Subcommand)]
pub enum StockCommand {
    Get(SymbolArg),
    Warnings(SymbolArg),
    Search(SymbolsArg),
}

#[derive(Debug, Args)]
pub struct MarketArgs {
    #[command(subcommand)]
    pub command: MarketCommand,
}

#[derive(Debug, Subcommand)]
pub enum MarketCommand {
    ExchangeRate,
    Calendar(CalendarArgs),
}

#[derive(Debug, Args)]
pub struct CalendarArgs {
    #[command(subcommand)]
    pub command: CalendarCommand,
}

#[derive(Debug, Subcommand)]
pub enum CalendarCommand {
    Kr,
    Us,
}

#[derive(Debug, Args)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub command: AccountCommand,
}

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    List,
    Use(AccountUseArgs),
}

#[derive(Debug, Args)]
pub struct AccountUseArgs {
    pub account_seq: u64,
}

#[derive(Debug, Args)]
pub struct SymbolArg {
    pub symbol: String,
}

#[derive(Debug, Args)]
pub struct SymbolsArg {
    #[arg(long)]
    pub symbols: String,
}
```

- [ ] **Step 4: Implement rendering and runtime for config first**

Create `rust/toss-cli/src/render.rs`:

```rust
use anyhow::Result;
use std::io::Write;
use unicode_width::UnicodeWidthStr;

pub fn write_key_values(writer: &mut dyn Write, rows: &[(&str, String)]) -> Result<()> {
    let width = rows.iter().map(|(key, _)| UnicodeWidthStr::width(*key)).max().unwrap_or(0);
    for (key, value) in rows {
        writeln!(writer, "{key:<width$}  {value}", width = width)?;
    }
    Ok(())
}
```

Create `rust/toss-cli/src/runtime.rs`:

```rust
use std::io::Write;

use anyhow::Result;
use serde::Serialize;
use serde_json::json;
use toss_core::config::{self, AppConfig};
use toss_core::TossError;

use crate::cli::{self, OutputFormat};
use crate::render;

#[derive(Debug, Serialize)]
struct SuccessEnvelope<'a, T> {
    ok: bool,
    command: &'a str,
    data: T,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
    ok: bool,
    command: &'a str,
    error: ErrorOutput,
}

#[derive(Debug, Serialize)]
struct ErrorOutput {
    kind: &'static str,
    code: Option<String>,
    message: String,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
}

pub async fn run(cli: cli::Cli, writer: &mut dyn Write) -> Result<()> {
    let command = cli.command.name();
    let app_config = config::load(cli.config.as_deref(), cli.account.as_deref())?;
    match cli.command {
        cli::Command::Config => run_config(&cli, command, &app_config, writer),
        cli::Command::Account(args) => match args.command {
            cli::AccountCommand::Use(args) => {
                let path = config::save_account_seq(cli.config.as_deref(), args.account_seq)?;
                write_output(&cli, command, json!({ "config_path": path, "account_seq": args.account_seq }), writer)
            }
            cli::AccountCommand::List => Err(anyhow::anyhow!("network commands are implemented in Task 5")),
        },
        _ => Err(anyhow::anyhow!("network commands are implemented in Task 5")),
    }
}

fn run_config(cli: &cli::Cli, command: &str, app_config: &AppConfig, writer: &mut dyn Write) -> Result<()> {
    let data = json!({
        "client_id": mask_client_id(&app_config.client_id),
        "account_seq": app_config.account_seq,
    });
    write_output(cli, command, data, writer)
}

fn write_output<T: Serialize>(cli: &cli::Cli, command: &str, data: T, writer: &mut dyn Write) -> Result<()> {
    match cli.output_format() {
        OutputFormat::Json => {
            serde_json::to_writer(writer, &SuccessEnvelope { ok: true, command, data })?;
            writeln!(writer)?;
        }
        OutputFormat::Text => {
            let value = serde_json::to_value(data)?;
            if command == "config" {
                render::write_key_values(writer, &[
                    ("client_id", value["client_id"].as_str().unwrap_or("-").to_string()),
                    ("account_seq", value["account_seq"].as_u64().map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())),
                ])?;
            } else {
                serde_json::to_writer_pretty(writer, &value)?;
                writeln!(writer)?;
            }
        }
    }
    Ok(())
}

pub fn write_json_error(writer: &mut dyn Write, command: &str, err: &anyhow::Error) -> Result<()> {
    let error = classify_error(err);
    serde_json::to_writer(writer, &ErrorEnvelope { ok: false, command, error })?;
    writeln!(writer)?;
    Ok(())
}

fn classify_error(err: &anyhow::Error) -> ErrorOutput {
    if let Some(toss) = err.downcast_ref::<TossError>() {
        match toss {
            TossError::Config(message) => return ErrorOutput { kind: "config", code: None, message: message.clone(), request_id: None },
            TossError::Auth(message) => return ErrorOutput { kind: "auth", code: None, message: message.clone(), request_id: None },
            TossError::Api { code, message, request_id, .. } => return ErrorOutput { kind: "api", code: code.clone(), message: message.clone(), request_id: request_id.clone() },
            TossError::RateLimit { message, request_id, .. } => return ErrorOutput { kind: "rate_limit", code: Some("rate-limit-exceeded".to_string()), message: message.clone(), request_id: request_id.clone() },
            TossError::Runtime(message) => return ErrorOutput { kind: "runtime", code: None, message: message.clone(), request_id: None },
            TossError::Io(_) | TossError::Yaml(_) | TossError::Json(_) | TossError::Http(_) => {}
        }
    }
    ErrorOutput { kind: "runtime", code: None, message: err.to_string(), request_id: None }
}

fn mask_client_id(client_id: &str) -> String {
    if client_id.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****{}", &client_id[..4], &client_id[client_id.len() - 4..])
}
```

Create `rust/toss-cli/src/main.rs`:

```rust
use clap::Parser;
use toss_cli::cli::{Cli, OutputFormat};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let command = cli.command.name();
    let output = cli.output_format();
    let mut stdout = std::io::stdout();
    if let Err(error) = toss_cli::runtime::run(cli, &mut stdout).await {
        match output {
            OutputFormat::Json => {
                let _ = toss_cli::runtime::write_json_error(&mut stdout, command, &error);
            }
            OutputFormat::Text => eprintln!("{error}"),
        }
        std::process::exit(1);
    }
}
```

- [ ] **Step 5: Run CLI tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli
```

Expected: parser and config binary smoke tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-cli/Cargo.toml rust/toss-cli/src/lib.rs rust/toss-cli/src/cli.rs rust/toss-cli/src/render.rs rust/toss-cli/src/runtime.rs rust/toss-cli/src/main.rs rust/toss-cli/tests/cli_smoke.rs
git commit -m "feat: add toss cli parser"
```

---

### Task 5: Wire Read-only Commands

**Files:**
- Modify: `rust/toss-cli/src/runtime.rs`
- Modify: `rust/toss-cli/tests/cli_smoke.rs`

**Interfaces:**
- Consumes: `toss_core::client::TossClient<ReqwestTransport>`
- Produces: all Phase 1 commands dispatch to core wrappers.

- [ ] **Step 1: Add parser coverage for remaining command groups**

Append to `rust/toss-cli/tests/cli_smoke.rs`:

```rust
use toss_cli::cli::{CalendarCommand, ChartCommand, MarketCommand, StockCommand};

#[test]
fn parses_chart_candles_command() {
    let cli = Cli::parse_from(["toss", "chart", "candles", "AAPL", "--interval", "1d", "--from", "2026-01-01"]);
    match cli.command {
        Command::Chart(args) => match args.command {
            ChartCommand::Candles(args) => {
                assert_eq!(args.symbol, "AAPL");
                assert_eq!(args.interval, "1d");
                assert_eq!(args.from.as_deref(), Some("2026-01-01"));
            }
        },
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_stock_and_market_commands() {
    let stock = Cli::parse_from(["toss", "stock", "search", "--symbols", "005930,AAPL"]);
    match stock.command {
        Command::Stock(args) => match args.command {
            StockCommand::Search(args) => assert_eq!(args.symbols, "005930,AAPL"),
            other => panic!("unexpected stock command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }

    let market = Cli::parse_from(["toss", "market", "calendar", "kr"]);
    match market.command {
        Command::Market(args) => match args.command {
            MarketCommand::Calendar(args) => match args.command {
                CalendarCommand::Kr => {}
                other => panic!("unexpected calendar command: {other:?}"),
            },
            other => panic!("unexpected market command: {other:?}"),
        },
        other => panic!("unexpected command: {other:?}"),
    }
}
```

- [ ] **Step 2: Run CLI tests to verify the new tests pass before dispatch changes**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli parses_chart_candles_command parses_stock_and_market_commands
```

Expected: parser tests pass; if Rust test filtering with two names does not match both, run `cargo test --manifest-path rust/Cargo.toml -p toss-cli parses_` and confirm both new tests pass.

- [ ] **Step 3: Replace network command dispatch**

Modify imports in `rust/toss-cli/src/runtime.rs`:

```rust
use toss_core::{account, asset, market_data, market_info, stock_info};
use toss_core::client::TossClient;
```

Replace the `match cli.command` body in `run` with:

```rust
    match cli.command {
        cli::Command::Config => run_config(&cli, command, &app_config, writer),
        cli::Command::Auth(args) => match args.command {
            cli::AuthCommand::Token => {
                let client = TossClient::new(app_config)?;
                client.check_token().await?;
                write_output(&cli, command, json!({ "token_check": "ok" }), writer)
            }
        },
        cli::Command::Price(args) => {
            let client = TossClient::new(app_config)?;
            let symbols = args.symbols.as_deref().unwrap_or(&args.symbol);
            let value = market_data::prices(&client, symbols).await?;
            write_output(&cli, command, value, writer)
        }
        cli::Command::Quote(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
                cli::QuoteCommand::Orderbook(arg) => market_data::orderbook(&client, &arg.symbol).await?,
                cli::QuoteCommand::Trades(arg) => market_data::trades(&client, &arg.symbol).await?,
                cli::QuoteCommand::Limits(arg) => market_data::price_limits(&client, &arg.symbol).await?,
            };
            write_output(&cli, command, value, writer)
        }
        cli::Command::Chart(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
                cli::ChartCommand::Candles(args) => {
                    let mut query = vec![
                        ("symbol".to_string(), args.symbol),
                        ("interval".to_string(), args.interval),
                    ];
                    if let Some(from) = args.from { query.push(("from".to_string(), from)); }
                    if let Some(to) = args.to { query.push(("to".to_string(), to)); }
                    market_data::candles(&client, query).await?
                }
            };
            write_output(&cli, command, value, writer)
        }
        cli::Command::Stock(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
                cli::StockCommand::Get(arg) => stock_info::stocks(&client, &arg.symbol).await?,
                cli::StockCommand::Warnings(arg) => stock_info::warnings(&client, &arg.symbol).await?,
                cli::StockCommand::Search(arg) => stock_info::stocks(&client, &arg.symbols).await?,
            };
            write_output(&cli, command, value, writer)
        }
        cli::Command::Market(args) => {
            let client = TossClient::new(app_config)?;
            let value = match args.command {
                cli::MarketCommand::ExchangeRate => market_info::exchange_rate(&client).await?,
                cli::MarketCommand::Calendar(args) => match args.command {
                    cli::CalendarCommand::Kr => market_info::kr_calendar(&client).await?,
                    cli::CalendarCommand::Us => market_info::us_calendar(&client).await?,
                },
            };
            write_output(&cli, command, value, writer)
        }
        cli::Command::Account(args) => match args.command {
            cli::AccountCommand::Use(args) => {
                let path = config::save_account_seq(cli.config.as_deref(), args.account_seq)?;
                write_output(&cli, command, json!({ "config_path": path, "account_seq": args.account_seq }), writer)
            }
            cli::AccountCommand::List => {
                let client = TossClient::new(app_config)?;
                let value = account::list(&client).await?;
                write_output(&cli, command, value, writer)
            }
        },
        cli::Command::Holdings => {
            let client = TossClient::new(app_config)?;
            let value = asset::holdings(&client).await?;
            write_output(&cli, command, value, writer)
        }
    }
```

- [ ] **Step 4: Run CLI tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml -p toss-cli
```

Expected: CLI parser and config smoke tests pass. Network commands are not exercised by binary smoke tests.

- [ ] **Step 5: Run all workspace tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add rust/toss-cli/src/runtime.rs rust/toss-cli/tests/cli_smoke.rs
git commit -m "feat: wire read only commands"
```

---

### Task 6: Documentation and Final Verification

**Files:**
- Create: `README.md`

**Interfaces:**
- Consumes: Phase 1 binary behavior.
- Produces: user-facing usage documentation.

- [ ] **Step 1: Write README**

Create `README.md`:

```markdown
# tossinvest-cli

Rust CLI for Toss Securities Open API. The binary name is `toss`.

## Install

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --release --bin toss
install -m 755 rust/target/release/toss ~/.local/bin/toss
```

## Config

Default config path:

```text
~/.config/tossinvest/config.yaml
```

Example:

```yaml
client_id: "issued-client-id"
client_secret: "issued-client-secret"
account_seq: 1
```

Environment overrides:

```bash
export TOSSINVEST_CLIENT_ID="issued-client-id"
export TOSSINVEST_CLIENT_SECRET="issued-client-secret"
export TOSSINVEST_ACCOUNT_SEQ="1"
```

## Commands

```bash
toss config
toss --json config

toss price 005930
toss price AAPL
toss quote orderbook AAPL
toss quote trades AAPL
toss quote limits 005930
toss chart candles AAPL --interval 1d

toss stock get AAPL
toss stock warnings 005930
toss stock search --symbols 005930,AAPL

toss market exchange-rate
toss market calendar kr
toss market calendar us

toss account list
toss account use 1
toss holdings
```

## Output

Use `--json` or `--output json` for automation. Successful JSON output uses:

```json
{"ok":true,"command":"price","data":{}}
```

Error JSON output uses:

```json
{"ok":false,"command":"price","error":{"kind":"api","code":"stock-not-found","message":"...","requestId":"..."}}
```

## Safety

Phase 1 is read-only. Order creation, modification, and cancellation are intentionally not exposed yet.
The CLI never prints `client_secret` or access tokens in normal output.
```

- [ ] **Step 2: Run formatter**

Run:

```bash
cargo fmt --manifest-path rust/Cargo.toml
```

Expected: command exits successfully.

- [ ] **Step 3: Run all tests**

Run:

```bash
cargo test --manifest-path rust/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 4: Build binary**

Run:

```bash
cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss
```

Expected: build succeeds.

- [ ] **Step 5: Smoke-test config command**

Run:

```bash
cargo run --manifest-path rust/Cargo.toml -p toss-cli --bin toss -- --config /tmp/tossinvest-empty-config.yaml --json config
```

Expected: command succeeds and prints JSON with `"ok":true`, masked `client_id`, and no secret.

- [ ] **Step 6: Commit**

Run:

```bash
git add README.md rust/Cargo.toml rust/toss-core rust/toss-cli
git commit -m "docs: document phase one cli"
```

---

## Self-Review

Spec coverage:

- Phase 1 config/auth/read-only commands are covered by Tasks 1-6.
- Stable JSON success/error envelopes are covered by Task 4.
- Account selection behavior is covered by Tasks 1, 3, and 4.
- Secret masking and token suppression are covered by Tasks 2, 4, and 6.
- Mutating order commands are excluded from this plan by design and remain Phase 3.
- Typed-model completeness is excluded from this plan by design and remains Phase 2.

Placeholder scan:

- The plan contains concrete paths, command lines, expected outputs, and code blocks for implementation steps.
- The plan does not use deferred implementation markers.

Type consistency:

- `AppConfig`, `TokenManager<T>`, `Transport`, `TossClient<T>`, and CLI command names are introduced before use.
- Endpoint wrappers consistently return `serde_json::Value` for Phase 1.
