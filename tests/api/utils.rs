pub fn assert_redirect_is_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);
    let location_header = response
        .headers()
        .get("Location")
        .expect("Could not find Location header");

    assert_eq!(location_header, location);
}
