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
            "_old_password": Uuid::new_v4().to_string(),
            "_new_password": &new_password,
            "_new_password_confirm": &new_password,
        }))
        .await;

    assert_redirect_is_to(&response, "/login");
}
