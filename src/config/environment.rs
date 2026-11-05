use std::env::{self, VarError};
use std::path::PathBuf;
use std::str::FromStr;

use log::debug;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Production,
    Development,
    Testing,
}

impl Environment {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Development => "development",
            Self::Testing => "testing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid ENVIRONMENT value '{value}': expected 'production', 'development', or 'testing' (case-insensitive)")]
pub struct InvalidEnvironmentError {
    value: String,
}

impl FromStr for Environment {
    type Err = InvalidEnvironmentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_lowercase() {
            "production" => Ok(Self::Production),
            "development" => Ok(Self::Development),
            "testing" => Ok(Self::Testing),
            _ => Err(InvalidEnvironmentError {
                value: s.to_owned(),
            }),
        }
    }
}

const ENVIRONMENT_VARIABLE: &str = "ENVIRONMENT";

/// Resolves the target environment from the `ENVIRONMENT` variable.
///
/// The crate's single resolution path: no other code may read the variable
/// directly. Absent defaults to development; invalid (including non-UTF-8)
/// fails fast so a typo can never silently target the development database
/// and parquet tree.
pub fn resolve_environment() -> Result<Environment, InvalidEnvironmentError> {
    resolve_from_raw(env::var(ENVIRONMENT_VARIABLE))
}

/// Takes the raw env-var result so the decision logic is testable without
/// mutating process-global state.
fn resolve_from_raw(raw: Result<String, VarError>) -> Result<Environment, InvalidEnvironmentError> {
    match raw {
        Ok(value) => value.parse(),
        Err(VarError::NotPresent) => {
            debug!("{ENVIRONMENT_VARIABLE} is not set; defaulting to 'development'");
            Ok(Environment::Development)
        }
        Err(VarError::NotUnicode(value)) => Err(InvalidEnvironmentError {
            value: value.to_string_lossy().into_owned(),
        }),
    }
}

#[derive(Debug)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl DbConfig {
    pub fn from_env(env: Environment) -> Self {
        let port = match env {
            Environment::Production => 5432,
            Environment::Development => 54320,
            Environment::Testing => 54321,
        };
        let database = match env {
            Environment::Production => "db-production",
            Environment::Development => "db-development",
            Environment::Testing => "db-testing",
        };
        Self {
            host: "host.docker.internal".into(),
            port,
            user: "postgres".into(),
            password: "postgres".into(),
            database: database.into(),
        }
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://{user}:{password}@{host}:{port}/{database}",
            user = self.user,
            password = self.password,
            host = self.host,
            port = self.port,
            database = self.database,
        )
    }
}

const PARQUET_FILES_PATH: &str = "/app/parquet_files";

pub fn parquet_base_dir(env: Environment) -> PathBuf {
    parquet_root_dir(env).join("strategy")
}

pub fn parquet_root_dir(env: Environment) -> PathBuf {
    PathBuf::from(PARQUET_FILES_PATH).join(env.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn environment_from_str_parses_valid_values() {
        assert_eq!(
            "production".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert_eq!(
            "development".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!(
            "testing".parse::<Environment>().unwrap(),
            Environment::Testing
        );
        assert_eq!(
            "Production".parse::<Environment>().unwrap(),
            Environment::Production
        );
    }

    #[test]
    fn environment_from_str_fails_on_invalid_values() {
        assert!("invalid".parse::<Environment>().is_err());
        assert!("".parse::<Environment>().is_err());
    }

    #[test]
    fn environment_from_str_error_names_accepted_values() {
        let err = "produciton".parse::<Environment>().unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid ENVIRONMENT value 'produciton': expected 'production', 'development', or \
             'testing' (case-insensitive)"
        );
    }

    #[test]
    fn dbconfig_from_env_returns_expected_host_and_port() {
        let prod = DbConfig::from_env(Environment::Production);
        assert_eq!(prod.host, "host.docker.internal");
        assert_eq!(prod.port, 5432);

        let dev = DbConfig::from_env(Environment::Development);
        assert_eq!(dev.host, "host.docker.internal");
        assert_eq!(dev.port, 54320);

        let test = DbConfig::from_env(Environment::Testing);
        assert_eq!(test.host, "host.docker.internal");
        assert_eq!(test.port, 54321);
    }

    #[test]
    fn dbconfig_from_env_returns_correct_config() {
        let env = Environment::from_str("production").unwrap();
        let config = DbConfig::from_env(env);
        assert_eq!(config.database, "db-production");

        let env = Environment::from_str("development").unwrap();
        let config = DbConfig::from_env(env);
        assert_eq!(config.database, "db-development");

        let env = Environment::from_str("testing").unwrap();
        let config = DbConfig::from_env(env);
        assert_eq!(config.database, "db-testing");
    }

    #[test]
    fn dbconfig_connection_string_generates_correct_string() {
        let config = DbConfig::from_env(Environment::Production);
        let expected = "postgresql://postgres:postgres@host.docker.internal:5432/db-production";
        assert_eq!(config.connection_string(), expected);
    }

    // The former `application_layer_creates_env_explicitly` test was removed:
    // it mutated process-global env vars (`set_var`/`remove_var`), which races
    // with parallel test threads, and its coverage is fully subsumed by the
    // parse tests above plus the `resolve_from_raw_*` tests below.

    #[test]
    fn resolve_from_raw_accepts_each_environment() {
        assert_eq!(
            resolve_from_raw(Ok("production".to_owned())).unwrap(),
            Environment::Production
        );
        assert_eq!(
            resolve_from_raw(Ok("Development".to_owned())).unwrap(),
            Environment::Development
        );
        assert_eq!(
            resolve_from_raw(Ok("testing".to_owned())).unwrap(),
            Environment::Testing
        );
    }

    #[test]
    fn resolve_from_raw_absent_defaults_to_development() {
        assert_eq!(
            resolve_from_raw(Err(VarError::NotPresent)).unwrap(),
            Environment::Development
        );
    }

    #[test]
    fn resolve_from_raw_invalid_value_fails_fast() {
        let err = resolve_from_raw(Ok("produciton".to_owned())).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid ENVIRONMENT value 'produciton': expected 'production', 'development', or \
             'testing' (case-insensitive)"
        );
    }

    #[test]
    fn resolve_from_raw_non_unicode_value_fails_fast() {
        let err =
            resolve_from_raw(Err(VarError::NotUnicode(OsString::from("production")))).unwrap_err();
        assert_eq!(
            err,
            InvalidEnvironmentError {
                value: "production".to_owned()
            }
        );
    }

    #[test]
    fn parquet_base_dir_development() {
        let dir = parquet_base_dir(Environment::Development);
        assert_eq!(dir, Path::new("/app/parquet_files/development/strategy"));
    }

    #[test]
    fn parquet_base_dir_production() {
        let dir = parquet_base_dir(Environment::Production);
        assert_eq!(dir, Path::new("/app/parquet_files/production/strategy"));
    }

    #[test]
    fn parquet_base_dir_testing() {
        let dir = parquet_base_dir(Environment::Testing);
        assert_eq!(dir, Path::new("/app/parquet_files/testing/strategy"));
    }

    #[test]
    fn parquet_root_dir_development() {
        let dir = parquet_root_dir(Environment::Development);
        assert_eq!(dir, Path::new("/app/parquet_files/development"));
    }

    #[test]
    fn parquet_root_dir_production() {
        let dir = parquet_root_dir(Environment::Production);
        assert_eq!(dir, Path::new("/app/parquet_files/production"));
    }

    #[test]
    fn parquet_root_dir_testing() {
        let dir = parquet_root_dir(Environment::Testing);
        assert_eq!(dir, Path::new("/app/parquet_files/testing"));
    }
}
