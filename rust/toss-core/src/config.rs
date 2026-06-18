use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, TossError};

#[derive(Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub client_id: String,
    pub client_secret: String,
    pub account_seq: Option<u64>,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &"[redacted]")
            .field("account_seq", &self.account_seq)
            .finish()
    }
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
    let account_seq = if let Some(raw) = account_override {
        Some(parse_account_seq(raw)?)
    } else if let Ok(raw) = env::var("TOSSINVEST_ACCOUNT_SEQ") {
        Some(parse_account_seq(&raw)?)
    } else {
        file.account_seq
    };

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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{load, read_file_config, save_account_seq, AppConfig};

    #[test]
    fn debug_redacts_client_secret() {
        let config = AppConfig {
            client_id: "client-file".to_string(),
            client_secret: "super-secret".to_string(),
            account_seq: Some(9),
        };
        let debug = format!("{config:?}");

        assert!(!debug.contains("super-secret"), "{debug}");
        assert!(debug.contains("client_id: \"client-file\""), "{debug}");
        assert!(debug.contains("client_secret: \"[redacted]\""), "{debug}");
        assert!(debug.contains("account_seq: Some(9)"), "{debug}");
    }

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
