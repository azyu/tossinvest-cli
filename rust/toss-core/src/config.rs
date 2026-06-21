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
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_seq: Option<u64>,
}

#[derive(Debug, Default)]
pub struct ConfigUpdate {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub account_seq: Option<Option<u64>>,
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
        Some(parse_account_seq(raw, TossError::Validation)?)
    } else if let Ok(raw) = env::var("TOSSINVEST_ACCOUNT_SEQ") {
        Some(parse_account_seq(&raw, TossError::Config)?)
    } else {
        validate_account_seq(file.account_seq, TossError::Config)?
    };

    Ok(AppConfig {
        client_id,
        client_secret,
        account_seq,
    })
}

pub fn save_config(config_path: Option<&Path>, update: ConfigUpdate) -> Result<PathBuf> {
    let path = default_config_path(config_path)?;
    let mut file = read_file_config(&path, true)?;
    if let Some(client_id) = update.client_id {
        file.client_id = Some(client_id);
    }
    if let Some(client_secret) = update.client_secret {
        file.client_secret = Some(client_secret);
    }
    if let Some(account_seq) = update.account_seq {
        file.account_seq = validate_account_seq(account_seq, TossError::Config)?;
    }
    write_file_config(&path, &file)?;
    Ok(path)
}

pub fn save_account_seq(config_path: Option<&Path>, account_seq: u64) -> Result<PathBuf> {
    let account_seq = validate_account_seq(Some(account_seq), TossError::Config)?;
    let path = default_config_path(config_path)?;
    let mut file = read_file_config(&path, true)?;
    file.account_seq = account_seq;
    write_file_config(&path, &file)?;
    Ok(path)
}

fn parse_account_seq(raw: &str, kind: fn(String) -> TossError) -> Result<u64> {
    let value = raw
        .parse::<u64>()
        .map_err(|_| kind(format!("invalid account sequence: {raw}")))?;
    validate_account_seq(Some(value), kind).map(|value| value.expect("validated input is Some"))
}

fn validate_account_seq(
    account_seq: Option<u64>,
    kind: fn(String) -> TossError,
) -> Result<Option<u64>> {
    if let Some(value) = account_seq
        && value > i64::MAX as u64
    {
        return Err(kind(format!("invalid account sequence: {value}")));
    }
    Ok(account_seq)
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

fn write_file_config(path: &Path, file: &FileConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        restrict_config_dir(parent)?;
    }
    let payload = serde_yaml::to_string(file)?;
    write_config_file(path, payload.as_bytes())?;
    Ok(())
}

#[cfg(unix)]
fn restrict_config_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn restrict_config_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn write_config_file(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| std::borrow::Cow::Borrowed("config.yaml"));
    let pid = std::process::id();

    for attempt in 0..16u32 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_path = dir.join(format!(".{file_name}.{pid}.{nanos}.{attempt}.tmp"));
        let mut temp = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temp_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };

        let result = (|| {
            temp.write_all(payload)?;
            temp.sync_all()?;
            drop(temp);
            fs::rename(&temp_path, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        return result;
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "unable to allocate config temp file",
    ))
}

#[cfg(not(unix))]
fn write_config_file(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    fs::write(path, payload)
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

    use super::{AppConfig, ConfigUpdate, load, read_file_config, save_account_seq, save_config};
    use crate::error::TossError;

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
    fn rejects_invalid_cli_account_override_as_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(
            &path,
            r#"
client_id: "client-file"
client_secret: "secret-file"
"#,
        )
        .unwrap();

        let err = load(Some(&path), Some("abc")).unwrap_err();
        assert!(
            matches!(err, TossError::Validation(ref message) if message.contains("invalid account sequence: abc")),
            "{err}"
        );
    }

    #[test]
    fn rejects_account_sequence_outside_openapi_int64_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(
            &path,
            r#"
client_id: "client-file"
client_secret: "secret-file"
account_seq: 9223372036854775808
"#,
        )
        .unwrap();

        let err = load(Some(&path), None).unwrap_err();
        assert!(
            matches!(err, TossError::Config(ref message) if message.contains("invalid account sequence")),
            "{err}"
        );

        let err = load(Some(&path), Some("9223372036854775808")).unwrap_err();
        assert!(
            matches!(err, TossError::Validation(ref message) if message.contains("invalid account sequence")),
            "{err}"
        );
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
    #[test]
    fn saves_credentials_with_restrictive_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");

        let saved = save_config(
            Some(&path),
            ConfigUpdate {
                client_id: Some("client-new".to_string()),
                client_secret: Some("secret-new".to_string()),
                account_seq: Some(Some(7)),
            },
        )
        .unwrap();

        assert_eq!(saved, path);
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("client_id: client-new"));
        assert!(contents.contains("client_secret: secret-new"));
        assert!(contents.contains("account_seq: 7"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode & 0o077, 0, "config must not be group/world readable");
        }
    }

    #[test]
    fn save_config_preserves_unspecified_values_and_clears_account() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(
            &path,
            r#"client_id: "client-file"
client_secret: "secret-file"
account_seq: 42
"#,
        )
        .unwrap();

        save_config(
            Some(&path),
            ConfigUpdate {
                client_id: Some("client-updated".to_string()),
                client_secret: None,
                account_seq: Some(None),
            },
        )
        .unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("client_id: client-updated"));
        assert!(contents.contains("client_secret: secret-file"));
        assert!(!contents.contains("account_seq"));
    }
}
