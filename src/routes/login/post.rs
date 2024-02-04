use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{authentication, utils};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, db_pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, LoginError> {
    let credentials = authentication::Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    let user_id = authentication::validate_credentials(credentials, &db_pool)
        .await
        .map_err(|e| match e {
            authentication::AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            authentication::AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        utils::errors::error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }

    fn error_response(&self) -> HttpResponse {
        let encoded_error = urlencoding::Encoded::new(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((LOCATION, format!("/login?error={}", encoded_error)))
            .finish()
    }
}
