use crate::error::DistillError;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub api_base: String,
    pub model: String,
}

impl Config {
    pub fn resolve(
        cli_key: Option<String>,
        cli_base: Option<String>,
        cli_model: Option<String>,
    ) -> crate::error::Result<Self> {
        let api_key = cli_key.or_else(|| env::var("DISTILL_API_KEY").ok()).ok_or(
            DistillError::MissingConfig {
                field: "API key",
                env_var: "DISTILL_API_KEY",
                flag: "--api-key",
            },
        )?;

        let api_base = cli_base
            .or_else(|| env::var("DISTILL_API_BASE").ok())
            .ok_or(DistillError::MissingConfig {
                field: "API base URL",
                env_var: "DISTILL_API_BASE",
                flag: "--api-base",
            })?;

        let model = cli_model.or_else(|| env::var("DISTILL_MODEL").ok()).ok_or(
            DistillError::MissingConfig {
                field: "model name",
                env_var: "DISTILL_MODEL",
                flag: "--model",
            },
        )?;

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
        }
        let result = Config::resolve(None, None, None);
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
}
