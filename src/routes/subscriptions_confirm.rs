use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;
use std::fmt::{Debug, Formatter};
use uuid::Uuid;

use crate::utils;

#[derive(Deserialize)]
pub struct Params {
    subscription_token: String,
}

#[tracing::instrument("Confirming a pending subscriber", skip_all)]
pub async fn confirm(
    params: web::Query<Params>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmSubscriberError> {
    let subscriber_id = get_subscriber_id_from_token(&db_pool, &params.subscription_token)
        .await
        .context("Failed to retrieve the subscriber id associated with the provided token.")?
        .ok_or(ConfirmSubscriberError::UnauthorizedError)?;

    confirm_subscriber(&db_pool, subscriber_id)
        .await
        .context("Failed to confirm new subscriber")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument("Mark subscriber as confirmed", skip_all)]
async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            UPDATE subscription SET
                status = 'confirmed'
            WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(db_pool)
    .await?;
    Ok(())
}

#[tracing::instrument("Getting subscriber ID from subscription token", skip_all)]
async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
            SELECT subscription_id FROM subscription_token WHERE
            subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscription_id))
}

#[derive(thiserror::Error)]
pub enum ConfirmSubscriberError {
    #[error("There is no subscriber associated with the provided token.")]
    UnauthorizedError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for ConfirmSubscriberError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        utils::errors::error_chain_fmt(&self, f)
    }
}

impl actix_web::ResponseError for ConfirmSubscriberError {
    fn status_code(&self) -> StatusCode {
        match self {
            ConfirmSubscriberError::UnauthorizedError => StatusCode::UNAUTHORIZED,
            ConfirmSubscriberError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
