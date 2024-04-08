use uuid::Uuid;

use crate::helpers::spawn_app;
use crate::utils::assert_redirect_is_to;

#[tokio::test]
async fn you_must_be_logged_in_to_see_change_password_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_change_password().await;

    assert_redirect_is_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let test_app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();
    let response = test_app
        .post_change_password(&serde_json::json!({
            "old_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_confirm": &new_password,
        }))
        .await;

    assert_redirect_is_to(&response, "/login");
}

#[tokio::test]
async fn new_passwords_must_match() {
    let test_app = spawn_app().await;

    // Act - Part 1 - Login
    test_app
        .post_login(&serde_json::json!({
            "username": test_app.test_user.username,
            "password": test_app.test_user.password,
        }))
        .await;

    // Act - Part 2 - Try to change password
    let new_password = Uuid::new_v4().to_string();
    let new_password_confirm = Uuid::new_v4().to_string();
    let response = test_app
        .post_change_password(&serde_json::json!({
            "old_password": &test_app.test_user.password,
            "new_password": &new_password,
            "new_password_confirm": &new_password_confirm,
        }))
        .await;

    assert_redirect_is_to(&response, "/admin/password");

    // Act - Part 3 - Follow the redirect
    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains(
        "<i>You entered two different new passwords - \
         the field values must match.</i>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let test_app = spawn_app().await;

    // Act - Part 1 - Login
    test_app
        .post_login(&serde_json::json!({
            "username": test_app.test_user.username,
            "password": test_app.test_user.password,
        }))
        .await;

    // Act - Part 2 - Try to change password
    let new_password = Uuid::new_v4().to_string();
    let response = test_app
        .post_change_password(&serde_json::json!({
            "old_password": "random-old-password",
            "new_password": &new_password,
            "new_password_confirm": &new_password,
        }))
        .await;

    assert_redirect_is_to(&response, "/admin/password");

    // Act - Part 3 - Follow the redirect
    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains("<i>The current password is incorrect</i>"));
}
