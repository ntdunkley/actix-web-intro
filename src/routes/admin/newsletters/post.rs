use crate::authentication::UserId;
use actix_web::web::ReqData;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency;
use crate::utils;

const NEWSLETTER_PUBLISHED: &str = "The newsletter issue has been published!";

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
    name = "Publishing newsletters to confirmed subscribers",
    skip(form, db_pool, email_client),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let FormData {
        title,
        html_content,
        text_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: idempotency::IdempotencyKey =
        idempotency_key.try_into().map_err(utils::error_400)?;
    let saved_response = idempotency::get_saved_response(&db_pool, &idempotency_key, user_id.0)
        .await
        .map_err(utils::error_500)?;

    // Return early if we already have a saved response
    if let Some(saved_response) = saved_response {
        FlashMessage::info(NEWSLETTER_PUBLISHED).send();
        return Ok(saved_response);
    }

    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .map_err(utils::error_500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(utils::error_500)?;
            }
            Err(error) => {
                tracing::warn!(
                    // We record the error chain as a structured field
                    // on the log record.
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }
    FlashMessage::info(NEWSLETTER_PUBLISHED).send();

    let response = utils::see_other("/admin/newsletters");
    let response = idempotency::save_response(&db_pool, &idempotency_key, user_id.0, response)
        .await
        .map_err(utils::error_500)?;
    Ok(response)
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscription
        WHERE status = 'confirmed'
        "#
    )
    .fetch_all(db_pool)
    .await?
    .into_iter()
    .map(|row| match SubscriberEmail::parse(row.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // We just forward to the Display implementation of
        // the wrapped String.
        self.0.fmt(f)
    }
}
