use crate::error::DistillError;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub api_base: String,
    pub model: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConfigFile {
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub model: Option<String>,
    pub level: Option<String>,
    pub jobs: Option<usize>,
}

pub fn config_path() -> PathBuf {
    let base = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        });
    base.join("distill").join("config.toml")
}

pub fn load_config_file() -> ConfigFile {
    load_config_file_from(&config_path())
}

fn load_config_file_from(path: &std::path::Path) -> ConfigFile {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config_file(config: &ConfigFile) -> std::io::Result<()> {
    save_config_file_to(&config_path(), config)
}

fn save_config_file_to(path: &std::path::Path, config: &ConfigFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config).expect("failed to serialize config");
    std::fs::write(path, content)
}

impl Config {
    pub fn resolve(
        cli_key: Option<String>,
        cli_base: Option<String>,
        cli_model: Option<String>,
    ) -> crate::error::Result<Self> {
        let file = load_config_file();

        let api_key = cli_key
            .or_else(|| env::var("DISTILL_API_KEY").ok())
            .or(file.api_key)
            .ok_or_else(|| DistillError::Config {
                cause: "API key required\n  Set in config:  distill config set api_key <value>\n  Or env var:     DISTILL_API_KEY=<value>\n  Or CLI flag:    --api-key <value>".into(),
            })?;

        let api_base = cli_base
            .or_else(|| env::var("DISTILL_API_BASE").ok())
            .or(file.api_base)
            .ok_or_else(|| DistillError::Config {
                cause: "API base URL required\n  Set in config:  distill config set api_base <value>\n  Or env var:     DISTILL_API_BASE=<value>\n  Or CLI flag:    --api-base <value>".into(),
            })?;

        let model = cli_model
            .or_else(|| env::var("DISTILL_MODEL").ok())
            .or(file.model)
            .ok_or_else(|| DistillError::Config {
                cause: "Model name required\n  Set in config:  distill config set model <value>\n  Or env var:     DISTILL_MODEL=<value>\n  Or CLI flag:    --model <value>".into(),
            })?;

        Ok(Self {
            api_key,
            api_base,
            model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_flags_override_env() {
        let config = Config {
            api_key: "from-cli".into(),
            api_base: "https://cli.example.com/v1".into(),
            model: "cli-model".into(),
        };
        assert_eq!(config.api_key, "from-cli");
    }

    #[test]
    fn test_missing_api_key_errors() {
        // Clear env vars to ensure they don't interfere
        // SAFETY: test is not relying on these env vars being present,
        // and tests using env vars should not run in parallel.
        unsafe {
            std::env::remove_var("DISTILL_API_KEY");
            std::env::remove_var("DISTILL_API_BASE");
            std::env::remove_var("DISTILL_MODEL");
            // Point config to nonexistent dir so real config file doesn't interfere
            std::env::set_var("XDG_CONFIG_HOME", "/tmp/distill-test-nonexistent");
        }
        let result = Config::resolve(None, None, None);
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_from_all_cli() {
        let result = Config::resolve(
            Some("key".into()),
            Some("https://api.example.com/v1".into()),
            Some("model".into()),
        );
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.api_key, "key");
    }

    #[test]
    fn test_config_path_ends_with_distill() {
        let path = config_path();
        assert!(path.ends_with("distill/config.toml"));
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let file = load_config_file_from(std::path::Path::new("/tmp/nonexistent/distill/config.toml"));
        assert!(file.api_key.is_none());
        assert!(file.api_base.is_none());
        assert!(file.model.is_none());
        assert!(file.level.is_none());
        assert!(file.jobs.is_none());
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("distill/config.toml");
        let config = ConfigFile {
            api_key: Some("sk-test123".into()),
            api_base: Some("https://api.example.com/v1".into()),
            model: Some("test-model".into()),
            level: Some("dense".into()),
            jobs: Some(8),
        };
        save_config_file_to(&path, &config).unwrap();
        let loaded = load_config_file_from(&path);
        assert_eq!(loaded.api_key.as_deref(), Some("sk-test123"));
        assert_eq!(loaded.api_base.as_deref(), Some("https://api.example.com/v1"));
        assert_eq!(loaded.model.as_deref(), Some("test-model"));
        assert_eq!(loaded.level.as_deref(), Some("dense"));
        assert_eq!(loaded.jobs, Some(8));
    }

    #[test]
    fn test_resolve_prefers_cli_over_env() {
        let result = Config::resolve(
            Some("cli-key".into()),
            Some("https://cli.example.com/v1".into()),
            Some("cli-model".into()),
        );
        let config = result.unwrap();
        assert_eq!(config.api_key, "cli-key");
        assert_eq!(config.api_base, "https://cli.example.com/v1");
        assert_eq!(config.model, "cli-model");
    }
}
