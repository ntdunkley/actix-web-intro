use crate::domain::SubscriberEmail;
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;
use std::time::Duration;

pub struct EmailClient {
    http_client: reqwest::Client,
    base_url: String,
    sender_email: SubscriberEmail,
    auth_token: Secret<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender_email: SubscriberEmail,
        auth_token: Secret<String>,
        timeout: Duration,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Could not create EmailClient");
        Self {
            base_url,
            http_client,
            sender_email,
            auth_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient_email: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender_email.as_ref(),
            to: recipient_email.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        self.http_client
            .post(url)
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .json(&request_body) // also sets Content-Type header to application/json
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use std::time::Duration;
    use wiremock::http::Method;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            if let Ok(body) = request.body_json::<serde_json::Value>() {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            Duration::from_secs(1),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method(Method::Post))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        // Assert - Mock server's expectations will be evaluated when it goes out of scope
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_mail_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // Configure mock server to only respond after 3 minutes
        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_delay(Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }
}
