use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;

    let response = test_app
        .api_client
        .get(format!("{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to execute GET health_check");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
