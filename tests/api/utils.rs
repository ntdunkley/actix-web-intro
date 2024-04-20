use rand::distributions::Alphanumeric;
use rand::Rng;

pub fn assert_redirect_is_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);
    let location_header = response
        .headers()
        .get("Location")
        .expect("Could not find Location header");

    assert_eq!(location_header, location);
}

pub fn generate_random_text_of_length<'a>(length: u8) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length as usize)
        .map(char::from)
        .collect::<String>()
}
