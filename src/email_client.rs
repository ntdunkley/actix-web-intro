use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: reqwest::Client,
    base_url: String,
    sender_email: SubscriberEmail,
}

impl EmailClient {
    pub fn new(base_url: String, sender_email: SubscriberEmail) -> Self {
        Self {
            base_url,
            http_client: reqwest::Client::new(),
            sender_email,
        }
    }

    pub fn send_email(
        &self,
        recipient_email: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), String> {
        todo!()
    }
}
