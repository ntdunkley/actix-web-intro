use sqlx::PgPool;
use std::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry};
use tracing_subscriber::layer::SubscriberExt;
use zero2prod::config;
use zero2prod::startup;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    LogTracer::init().expect("Failed to set logger");
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".to_string(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let config = config::get_config().expect("Failed to read config file");
    let address = format!("127.0.0.1:{}", config.application_port);

    let listener = TcpListener::bind(address)
        .unwrap_or_else(|_| panic!("Could not bind port {}", config.application_port));

    let db_pool = PgPool::connect(config.database.connection_string().as_str())
        .await
        .expect("Could not open postgres connection");

    startup::run(listener, db_pool)?.await
}
