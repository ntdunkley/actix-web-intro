use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::config;
use zero2prod::startup;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod", "info", std::io::stdout);
    init_subscriber(subscriber);

    let config = config::get_config().expect("Failed to read config file");
    let address = format!("{}:{}", config.application.host, config.application.port);

    let listener = TcpListener::bind(address)
        .unwrap_or_else(|_| panic!("Could not bind port {}", config.application.port));

    let db_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(config.database.connection_string().expose_secret().as_str())
        .expect("Could not open postgres connection");

    startup::run(listener, db_pool)?.await
}
