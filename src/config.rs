use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub database_name: String,
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

pub enum Environment {
    LOCAL,
    PROD,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "{}/{}",
            self.connection_string_without_db().expose_secret(),
            self.database_name
        ))
    }

    pub fn connection_string_without_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        ))
    }
}

impl Environment {
    pub fn as_str(&self) -> &str {
        match self {
            Environment::LOCAL => "local",
            Environment::PROD => "prod",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Environment::LOCAL),
            "prod" => Ok(Environment::PROD),
            other => Err(format!(
                "{} is not a supported environment. \
                Use either '{}' or '{}'",
                other,
                Environment::LOCAL.as_str(),
                Environment::PROD.as_str()
            )),
        }
    }
}

pub fn get_config() -> Result<Settings, config::ConfigError> {
    let config_dir = std::env::current_dir()
        .expect("Failed to determine current directory")
        .join("config");

    // Detect running environment, defaulting to local
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".to_string())
        .try_into()
        .expect("Failed to parse app environment");
    let environment_config_filename = format!("{}.yaml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .add_source(config::File::from(
            config_dir.join(environment_config_filename),
        ))
        .build()?;
    settings.try_deserialize::<Settings>()
}
