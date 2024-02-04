use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::config::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    confirm, health_check, home, login, login_form, publish_newsletter, subscribe,
};

pub struct Application {
    port: u16,
    server: Server,
}

pub struct ApplicationBaseUrl(pub String);

impl Application {
    pub async fn build(config: Settings) -> Result<Self, std::io::Error> {
        let db_pool = get_db_pool(&config.database);

        let sender_email = config
            .email_client
            .sender_email()
            .expect("Could not parse sender email");

        let timeout = config.email_client.timeout();
        let email_client = EmailClient::new(
            config.email_client.base_url,
            sender_email,
            config.email_client.auth_token,
            timeout,
        );

        let address = format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, db_pool, email_client, config.application.base_url)?;

        Ok(Self { port, server })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    /*
    Use web::Data to wrap our connection pool in an ARC pointer.
    HttpServer::new below requires its closure to be cloneable.
     */
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

pub fn get_db_pool(config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}
