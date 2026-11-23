use std::env::{self, VarError};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use log::debug;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Production,
    Development,
    Testing,
    Alpha,
    Beta,
}

impl Environment {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Development => "development",
            Self::Testing => "testing",
            Self::Alpha => "alpha",
            Self::Beta => "beta",
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
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            _ => Err(InvalidEnvironmentError {
                value: s.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "invalid ENVIRONMENT value '{value}': expected 'production', 'development', 'testing', 'alpha', or 'beta' (case-insensitive)"
)]
pub struct InvalidEnvironmentError {
    value: String,
}

const ENVIRONMENT_VARIABLE: &str = "ENVIRONMENT";

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
            Environment::Alpha => 54322,
            Environment::Beta => 54323,
        };
        let database = match env {
            Environment::Production => "db-production",
            Environment::Development => "db-development",
            Environment::Testing => "db-testing",
            Environment::Alpha => "db-alpha",
            Environment::Beta => "db-beta",
        };
        // Intentional: the host and credentials are hard-coded. The engine runs
        // only locally, against PostgreSQL containers on the developer's machine,
        // never on a server, so there is no secret to keep out of source.
        Self {
            host: "host.docker.internal".into(),
            port,
            user: "postgres".into(),
            password: "postgres".into(),
            database: database.into(),
        }
    }
}

impl fmt::Debug for DbConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DbConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("password", &"<redacted>")
            .field("database", &self.database)
            .finish()
    }
}

const PARQUET_FILES_PATH: &str = "/app/parquet_files";

pub fn parquet_root_dir(env: Environment) -> PathBuf {
    PathBuf::from(PARQUET_FILES_PATH).join(env.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn environment_from_str_parses_each_variant_in_any_letter_case() {
        let cases = [
            ("production", Environment::Production),
            ("development", Environment::Development),
            ("testing", Environment::Testing),
            ("PRODUCTION", Environment::Production),
            ("DEVELOPMENT", Environment::Development),
            ("TESTING", Environment::Testing),
            ("alpha", Environment::Alpha),
            ("beta", Environment::Beta),
            ("ALPHA", Environment::Alpha),
            ("BETA", Environment::Beta),
            ("Production", Environment::Production),
            ("Development", Environment::Development),
        ];
        for (input, expected) in cases {
            assert_eq!(input.parse::<Environment>().unwrap(), expected, "{input}");
        }
    }

    #[test]
    fn environment_from_str_fails_on_invalid_values() {
        assert!("invalid".parse::<Environment>().is_err());
        assert!("".parse::<Environment>().is_err());
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
            "invalid ENVIRONMENT value 'produciton': expected 'production', 'development', \
             'testing', 'alpha', or 'beta' (case-insensitive)"
        );
    }

    #[test]
    fn dbconfig_from_env_targets_each_environment() {
        let cases = [
            (Environment::Production, 5432, "db-production"),
            (Environment::Development, 54320, "db-development"),
            (Environment::Testing, 54321, "db-testing"),
            (Environment::Alpha, 54322, "db-alpha"),
            (Environment::Beta, 54323, "db-beta"),
        ];
        for (env, port, database) in cases {
            let config = DbConfig::from_env(env);
            assert_eq!(config.port, port, "{env:?}");
            assert_eq!(config.database, database, "{env:?}");
        }
    }

    #[test]
    fn dbconfig_debug_redacts_the_password() {
        let config = DbConfig {
            host: "db.example".into(),
            port: 6543,
            user: "engine_role".into(),
            password: "s3cret-pw".into(),
            database: "db-staging".into(),
        };

        let debug = format!("{config:?}");

        assert!(!debug.contains("s3cret-pw"), "{debug}");
        assert!(debug.contains("db.example"), "{debug}");
    }

    #[test]
    fn resolve_from_raw_present_value_is_parsed() {
        assert_eq!(
            resolve_from_raw(Ok("production".to_owned())).unwrap(),
            Environment::Production
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
            "invalid ENVIRONMENT value 'produciton': expected 'production', 'development', \
             'testing', 'alpha', or 'beta' (case-insensitive)"
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
            (Environment::Production, "/app/parquet_files/production"),
            (Environment::Development, "/app/parquet_files/development"),
            (Environment::Testing, "/app/parquet_files/testing"),
            (Environment::Alpha, "/app/parquet_files/alpha"),
            (Environment::Beta, "/app/parquet_files/beta"),
        ];
        for (env, root) in cases {
            assert_eq!(parquet_root_dir(env), Path::new(root));
        }
    }
}
