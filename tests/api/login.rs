use crate::helpers::spawn_app;
use crate::utils::assert_redirect_is_to;

#[tokio::test]
async fn auth_error_flash_message_is_shown_when_login_fails() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });

    let response = app.post_login(&body).await;
    // Assert that we get redirected to /login
    assert_redirect_is_to(&response, "/login");

    // Get the HTML from GET /login
    let html_page = app.get_login_html().await;

    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));
}
