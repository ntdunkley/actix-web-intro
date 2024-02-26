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
    // Assert that we get redirected to /login and error flash message is there
    assert_redirect_is_to(&response, "/login");
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("Authentication failed"));

    // Refresh GET /login and assert error flash message is now gone
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains("Authentication failed"));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    let response = app.post_login(&body).await;
    assert_redirect_is_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", &app.test_user.username)));
}

#[tokio::test]
async fn you_must_be_logged_in_to_access_admin_dashboard() {
    let app = spawn_app().await;

    // If navigating directly to admin dashboard, user should be redirected to login page
    let response = app.get_admin_dashboard().await;
    assert_redirect_is_to(&response, "/login");
}
