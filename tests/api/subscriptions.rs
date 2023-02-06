use crate::helpers::spawn_app;
use reqwest::StatusCode;

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
    assert_eq!(response.status(), StatusCode::OK);

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
            response.status(),
            StatusCode::BAD_REQUEST,
            "The API did not return 400 when payload was {error_message}"
        );
    }
}

#[tokio::test]
async fn when_subscribe_with_fields_that_are_present_but_invalid_return_400() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=bryan%40gmail.com", "empty name"),
        ("name=bryan&email=", "empty email"),
        ("name=bry<an&email=bryan%40gmail.com", "invalid name"),
        ("name=bryan&email=brya<n%40gmail.com", "invalid email"),
    ];

    for (invalid_test_case, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_test_case)
            .send()
            .await
            .expect("Failed to execute POST subscribe");

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "The API did not return 400 when payload had an {error_message}"
        );
    }
}
