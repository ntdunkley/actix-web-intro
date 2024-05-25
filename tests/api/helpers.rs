use std::sync::Once;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

use zero2prod::config::DatabaseSettings;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::get_db_pool;
use zero2prod::startup::Application;
use zero2prod::telemetry;
use zero2prod::{config, issue_delivery_worker};

static TRACING: Once = Once::new();

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let issue_delivery_worker::ExecutionOutcome::EmptyQueue =
                issue_delivery_worker::try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
    }
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute POST subscribe")
    }

    pub async fn post_login<T>(&self, body: &T) -> reqwest::Response
    where
        T: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute POST login")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("{}/login", self.address))
            .send()
            .await
            .expect("Could not GET /login")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Could not GET /admin/dashboard")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/password", self.address))
            .send()
            .await
            .expect("Could not GET /admin/password")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("Failed to execute POST logout")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password<T>(&self, body: &T) -> reqwest::Response
    where
        T: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute POST change password")
    }

    pub async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/newsletters", self.address))
            .send()
            .await
            .expect("Could not GET /admin/newsletters")
    }

    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.unwrap()
    }

    pub async fn post_publish_newsletter<T: serde::Serialize>(
        &self,
        body: &T,
    ) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/newsletters", self.address))
            .form(body)
            .send()
            .await
            .expect("Could not POST /admin/newsletters")
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
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub async fn store(&self, db_pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        sqlx::query!(
            r#"
                INSERT INTO users(user_id, username, password) VALUES ($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            password_hash
        )
        .execute(db_pool)
        .await
        .expect("Failed to insert test user");
    }

    pub async fn login(&self, test_app: &TestApp) {
        test_app
            .post_login(&serde_json::json!({
                    "username": self.username,
                    "password": self.password
            }))
            .await;
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

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .expect("Could not create reqwest Client");

    tokio::spawn(application.run_until_stopped());

    let test_app = TestApp {
        address,
        port: application_port,
        db_pool: get_db_pool(&config.database),
        email_server,
        test_user: TestUser::generate(),
        api_client: client,
        email_client: config.email_client.client(),
    };
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}

async fn configure_db(settings: &DatabaseSettings) -> PgPool {
    // Create DB
    let mut connection = PgConnection::connect_with(&settings.without_db())
        .await
        .expect("Failed to connect to postgres");

    tracing::info!("Creating database with name {}", &settings.database_name);
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
