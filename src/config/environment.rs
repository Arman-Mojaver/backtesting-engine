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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid ENVIRONMENT value '{value}': expected 'production', 'development', or 'testing' (case-insensitive)")]
pub struct InvalidEnvironmentError {
    value: String,
}

const ENVIRONMENT_VARIABLE: &str = "ENVIRONMENT";

/// Resolves the target environment from the `ENVIRONMENT` variable.
pub fn resolve_environment() -> Result<Environment, InvalidEnvironmentError> {
    resolve_from_raw(env::var(ENVIRONMENT_VARIABLE))
}

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
    fn environment_from_str_is_case_insensitive_for_all_variants() {
        assert_eq!(
            "PRODUCTION".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert_eq!(
            "DEVELOPMENT".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!(
            "TESTING".parse::<Environment>().unwrap(),
            Environment::Testing
        );
    }

    #[test]
    fn environment_from_str_rejects_surrounding_whitespace() {
        assert!(" production".parse::<Environment>().is_err());
        assert!("production\n".parse::<Environment>().is_err());
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
    fn dbconfig_connection_string_targets_each_environment() {
        let cases = [
            (
                Environment::Production,
                "postgresql://postgres:postgres@host.docker.internal:5432/db-production",
            ),
            (
                Environment::Development,
                "postgresql://postgres:postgres@host.docker.internal:54320/db-development",
            ),
            (
                Environment::Testing,
                "postgresql://postgres:postgres@host.docker.internal:54321/db-testing",
            ),
        ];
        for (env, expected) in cases {
            assert_eq!(DbConfig::from_env(env).connection_string(), expected);
        }
    }

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
    fn parquet_dirs_are_scoped_per_environment() {
        let cases = [
            (
                Environment::Production,
                "/app/parquet_files/production",
                "/app/parquet_files/production/strategy",
            ),
            (
                Environment::Development,
                "/app/parquet_files/development",
                "/app/parquet_files/development/strategy",
            ),
            (
                Environment::Testing,
                "/app/parquet_files/testing",
                "/app/parquet_files/testing/strategy",
            ),
        ];
        for (env, root, base) in cases {
            assert_eq!(parquet_root_dir(env), Path::new(root));
            assert_eq!(parquet_base_dir(env), Path::new(base));
        }
    }
}
