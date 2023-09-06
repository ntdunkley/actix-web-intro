use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::Once;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::config;
use zero2prod::config::DatabaseSettings;
use zero2prod::startup::get_db_pool;
use zero2prod::startup::Application;
use zero2prod::telemetry;

static TRACING: Once = Once::new();

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute POST subscribe")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        let (username, password) = self.test_user().await;
        reqwest::Client::new()
            .post(format!("{}/newsletters", self.address))
            .basic_auth(username, Some(password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute POST subscribe")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();

            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_string();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }

    async fn test_user(&self) -> (String, String) {
        let row = sqlx::query!(
            r#"
                SELECT username, password FROM users LIMIT 1;
            "#
        )
        .fetch_one(&self.db_pool)
        .await
        .expect("Failed to retrieve test user");

        (row.username, row.password)
    }
}

pub async fn spawn_app() -> TestApp {
    TRACING.call_once(|| {
        let default_filter_level = "info";
        let subscriber_name = "zero2prod - test";

        if std::env::var("TEST_LOG").is_ok() {
            let subscriber =
                telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
            telemetry::init_subscriber(subscriber);
        } else {
            let subscriber =
                telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::sink);
            telemetry::init_subscriber(subscriber);
        }
    });

    // Launch a mock server to act as Postmark's API
    let email_server = MockServer::start().await;

    // Randomise config to ensure test isolation
    let config = {
        let mut c = config::get_config().expect("Failed to read config file");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    // Create and migrate the database
    configure_db(&config.database).await;

    // Launch the application as a background task
    let application = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let application_port = application.port();
    let address = format!("http://127.0.0.1:{}", application_port);

    tokio::spawn(application.run_until_stopped());

    let test_app = TestApp {
        address,
        port: application_port,
        db_pool: get_db_pool(&config.database),
        email_server,
    };
    add_test_user(&test_app.db_pool).await;
    test_app
}

async fn configure_db(settings: &DatabaseSettings) -> PgPool {
    // Create DB
    let mut connection = PgConnection::connect_with(&settings.without_db())
        .await
        .expect("Failed to connect to postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, settings.database_name).as_str())
        .await
        .expect("Failed to create database");

    // Migrate DB
    let db_pool = PgPool::connect_with(settings.with_db())
        .await
        .expect("Failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate database");
    db_pool
}

async fn add_test_user(db_pool: &PgPool) {
    sqlx::query!(
        r#"
        INSERT INTO users(user_id, username, password) VALUES ($1, $2, $3);
        "#,
        Uuid::new_v4(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string()
    )
    .execute(db_pool)
    .await
    .expect("Failed to create test user");
}
