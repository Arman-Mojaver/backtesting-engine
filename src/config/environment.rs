use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Environment {
    Production,
    Development,
    Testing,
}

impl FromStr for Environment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_lowercase() {
            "production" => Ok(Environment::Production),
            "development" => Ok(Environment::Development),
            "testing" => Ok(Environment::Testing),
            _ => Err(()),
        }
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
    pub fn production() -> Self {
        DbConfig {
            host: "host.docker.internal".into(),
            port: 5432,
            user: "postgres".into(),
            password: "postgres".into(),
            database: "db-production".into(),
        }
    }

    pub fn development() -> Self {
        DbConfig {
            host: "host.docker.internal".into(),
            port: 54320,
            user: "postgres".into(),
            password: "postgres".into(),
            database: "db-development".into(),
        }
    }

    pub fn testing() -> Self {
        DbConfig {
            host: "host.docker.internal".into(),
            port: 54321,
            user: "postgres".into(),
            password: "postgres".into(),
            database: "db-testing".into(),
        }
    }

    pub fn from_env(env: Environment) -> Self {
        match env {
            Environment::Production => DbConfig::production(),
            Environment::Development => DbConfig::development(),
            Environment::Testing => DbConfig::testing(),
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

/// The root directory for parquet files inside the container.
const PARQUET_FILES_PATH: &str = "/app/parquet_files";

pub fn parquet_base_dir(env: Environment) -> String {
    let subdirectory = match env {
        Environment::Production => "production",
        Environment::Development => "development",
        Environment::Testing => "testing",
    };
    format!("{PARQUET_FILES_PATH}/{subdirectory}/strategy")
}

pub fn parquet_root_dir(env: Environment) -> String {
    let subdirectory = match env {
        Environment::Production => "production",
        Environment::Development => "development",
        Environment::Testing => "testing",
    };
    format!("{PARQUET_FILES_PATH}/{subdirectory}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn set_env(value: &str) {
        env::set_var("ENVIRONMENT", value);
    }

    fn clear_env() {
        env::remove_var("ENVIRONMENT");
    }

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
    fn dbconfig_factory_methods_return_expected_values() {
        let prod = DbConfig::production();
        assert_eq!(prod.host, "host.docker.internal");
        assert_eq!(prod.port, 5432);

        let dev = DbConfig::development();
        assert_eq!(dev.host, "host.docker.internal");
        assert_eq!(dev.port, 54320);

        let test = DbConfig::testing();
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
        let config = DbConfig::production();
        let expected = "postgresql://postgres:postgres@host.docker.internal:5432/db-production";
        assert_eq!(config.connection_string(), expected);
    }

    #[test]
    fn application_layer_creates_env_explicitly() {
        set_env("development");
        let env_var = env::var("ENVIRONMENT").unwrap();
        let env = Environment::from_str(&env_var).unwrap();
        let config = DbConfig::from_env(env);

        assert_eq!(env, Environment::Development);
        assert_eq!(config.database, "db-development");

        clear_env();
    }

    #[test]
    fn parquet_base_dir_development() {
        let dir = parquet_base_dir(Environment::Development);
        assert_eq!(dir, "/app/parquet_files/development/strategy");
    }

    #[test]
    fn parquet_base_dir_production() {
        let dir = parquet_base_dir(Environment::Production);
        assert_eq!(dir, "/app/parquet_files/production/strategy");
    }

    #[test]
    fn parquet_base_dir_testing() {
        let dir = parquet_base_dir(Environment::Testing);
        assert_eq!(dir, "/app/parquet_files/testing/strategy");
    }

    #[test]
    fn parquet_root_dir_development() {
        let dir = parquet_root_dir(Environment::Development);
        assert_eq!(dir, "/app/parquet_files/development");
    }

    #[test]
    fn parquet_root_dir_production() {
        let dir = parquet_root_dir(Environment::Production);
        assert_eq!(dir, "/app/parquet_files/production");
    }

    #[test]
    fn parquet_root_dir_testing() {
        let dir = parquet_root_dir(Environment::Testing);
        assert_eq!(dir, "/app/parquet_files/testing");
    }
}
