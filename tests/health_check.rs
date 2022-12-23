use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use std::sync::Once;
use uuid::Uuid;
use zero2prod::config;
use zero2prod::config::DatabaseSettings;
use zero2prod::telemetry;

static TRACING: Once = Once::new();

struct TestApp {
    address: String,
    db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
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

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let mut config = config::get_config().expect("Failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();
    let db_pool = configure_db(&config.database).await;

    let server = zero2prod::startup::run(listener, db_pool.clone()).expect("Failed to spawn app");

    tokio::spawn(server);

    let address = format!("http://127.0.0.1:{}", port);

    TestApp { address, db_pool }
}

async fn configure_db(settings: &DatabaseSettings) -> PgPool {
    // Create DB
    let mut connection = PgConnection::connect(settings.connection_string_without_db().as_str())
        .await
        .expect("Failed to connect to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, settings.database_name).as_str())
        .await
        .expect("Failed to create database");

    // Migrate DB
    let db_pool = PgPool::connect(settings.connection_string().as_str())
        .await
        .expect("Failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate database");
    db_pool
}

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to execute GET health_check");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

#[tokio::test]
async fn when_subscribe_with_valid_form_data_return_200() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=bryan&email=bryan%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute POST subscribe");
    assert_eq!(response.status().as_u16(), 200);

    // Test that the entry was inserted into the database successfully
    let query = sqlx::query!("SELECT name, email FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions");
    assert_eq!(query.name, "bryan");
    assert_eq!(query.email, "bryan@gmail.com");
}

#[tokio::test]
async fn when_subscribe_with_invalid_form_data_return_400() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=bryan", "missing email"),
        ("email=bryan@gmail.com", "missing name"),
        ("", "missing name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute POST subscribe");

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not return 400 when payload was {}",
            error_message
        );
    }
}
