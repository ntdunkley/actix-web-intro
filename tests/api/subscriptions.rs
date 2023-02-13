use crate::helpers::spawn_app;
use reqwest::StatusCode;
use wiremock::http::Method;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn when_subscribe_with_valid_form_data_return_200() {
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method(Method::Post))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let body = "name=bryan&email=bryan%40gmail.com";
    let response = test_app.post_subscriptions(body.to_string()).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Test that the entry was inserted into the database successfully
    let query = sqlx::query!("SELECT name, email, status FROM subscription")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(query.name, "bryan");
    assert_eq!(query.email, "bryan@gmail.com");
    assert_eq!(query.status, "confirmed");
}

#[tokio::test]
async fn when_subscribe_with_invalid_form_data_return_400() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=bryan", "missing email"),
        ("email=bryan@gmail.com", "missing name"),
        ("", "missing name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(invalid_body.to_string()).await;

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
    let test_cases = vec![
        ("name=&email=bryan%40gmail.com", "empty name"),
        ("name=bryan&email=", "empty email"),
        ("name=bry<an&email=bryan%40gmail.com", "invalid name"),
        ("name=bryan&email=brya<n%40gmail.com", "invalid email"),
    ];

    for (invalid_test_case, error_message) in test_cases {
        let response = test_app
            .post_subscriptions(invalid_test_case.to_string())
            .await;

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "The API did not return 400 when payload had an {error_message}"
        );
    }
}

#[tokio::test]
async fn subscribe_with_valid_data_sends_email_confirmation() {
    let test_app = spawn_app().await;
    let body = "name=bryan&email=bryan%40gmail.com";

    Mock::given(path("/email"))
        .and(method(Method::Post))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.to_string()).await;

    // Mock asserts when dropped
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;
    let body = "name=bryan&email=bryan%40gmail.com";

    Mock::given(path("/email"))
        .and(method(Method::Post))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.to_string()).await;

    // Get the first intercepted request
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    // Parse the body as JSON
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_string()
    };

    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}
