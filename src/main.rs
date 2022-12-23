use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::config;
use zero2prod::startup;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod", "info", std::io::stdout);
    init_subscriber(subscriber);

    let config = config::get_config().expect("Failed to read config file");
    let address = format!("127.0.0.1:{}", config.application_port);

    let listener = TcpListener::bind(address)
        .unwrap_or_else(|_| panic!("Could not bind port {}", config.application_port));

    let db_pool = PgPool::connect(config.database.connection_string().as_str())
        .await
        .expect("Could not open postgres connection");

    startup::run(listener, db_pool)?.await
}
