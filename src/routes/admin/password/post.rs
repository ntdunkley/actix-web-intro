use crate::session_state::TypedSession;
use crate::utils;

use crate::authentication;
use crate::routes::admin;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct FormData {
    old_password: Secret<String>,
    new_password: Secret<String>,
    new_password_confirm: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    // User must be logged in
    let user_id = session.get_user_id().map_err(utils::error_500)?;
    if user_id.is_none() {
        return Ok(utils::see_other("/login"));
    }
    let user_id = user_id.unwrap();

    // New passwords must match
    if form.new_password.expose_secret() != form.new_password_confirm.expose_secret() {
        let flash_message_text = "You entered two different new passwords - \
                    the field values must match.";
        FlashMessage::error(flash_message_text).send();
        return Ok(utils::see_other("/admin/password"));
    }

    // Entered current password must be correct
    let username = admin::fetch_username(&user_id, &db_pool)
        .await
        .map_err(utils::error_500)?;
    let credentials = authentication::Credentials {
        username,
        password: form.0.old_password,
    };
    if let Err(e) = authentication::validate_credentials(credentials, &db_pool).await {
        return match e {
            authentication::AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect").send();
                Ok(utils::see_other("/admin/password"))
            }
            authentication::AuthError::UnexpectedError(e) => Err(utils::error_500(e)),
        };
    }
    todo!()
}
