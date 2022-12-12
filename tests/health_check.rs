use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::config;

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute GET health_check");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

#[tokio::test]
async fn when_subscribe_with_valid_form_data_return_200() {
    let address = spawn_app();
    let config = config::get_config().expect("Failed to read config file");
    let database_connection_str = config.database.connection_string();
    let mut connection = PgConnection::connect(&database_connection_str)
        .await
        .expect("Failed to connect to Postgres");

    let client = reqwest::Client::new();

    let body = "_name=bryan&_email=bryan%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute POST subscribe");

    assert_eq!(response.status().as_u16(), 200);

    // Test that entry was inserted into the database successfully
    let query = sqlx::query!("SELECT name, email FROM subscriptions")
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscriptions");
    assert_eq!(query.name, "bryan");
    assert_eq!(query.email, "bryan@gmail.com");
}

#[tokio::test]
async fn when_subscribe_with_invalid_form_data_return_400() {
    let address = spawn_app();
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=bryan", "missing email"),
        ("email=bryan@gmail.com", "missing name"),
        ("", "missing name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &address))
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

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::startup::run(listener).expect("Failed to spawn app");

    tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}
