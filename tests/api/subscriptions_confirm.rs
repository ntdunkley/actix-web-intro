use crate::helpers::spawn_app;
use reqwest::StatusCode;
use wiremock::http::Method;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn when_subscriber_confirms_email_without_token_then_reject_with_a_400() {
    // Given
    let test_app = spawn_app().await;

    // When
    let response = reqwest::get(format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn when_clicking_link_returned_by_subscribe_response_is_200() {
    // Given
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method(Method::Post))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let body = "name=bryan&email=bryan%40gmail.com";

    // When
    test_app.post_subscriptions(body.to_string()).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];

    let confirmation_links = test_app.get_confirmation_links(email_request);

    // Then
    let response = reqwest::get(confirmation_links.html).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
