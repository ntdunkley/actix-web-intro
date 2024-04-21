use crate::helpers::spawn_app;
use crate::utils::assert_redirect_is_to;

#[tokio::test]
async fn logout_clears_session_state() {
    let test_app = spawn_app().await;

    // Act - Part 1 - Login
    let response = test_app
        .post_login(&serde_json::json!({
            "username": test_app.test_user.username,
            "password": test_app.test_user.password,
        }))
        .await;
    assert_redirect_is_to(&response, "/admin/dashboard");

    // Act - Part 2 - Follow the redirect
    let html_page = test_app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", &test_app.test_user.username)));

    // Act - Part 3 - Logout
    let response = test_app.post_logout().await;
    assert_redirect_is_to(&response, "/login");

    // Act - Part 4 - Follow the redirect
    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains("You have successfully logged out"));

    // Act - Part 5 - Try to access admin dashboard
    let response = test_app.get_admin_dashboard().await;
    assert_redirect_is_to(&response, "/login");
}
