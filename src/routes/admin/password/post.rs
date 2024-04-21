use crate::utils;

use crate::authentication;
use crate::authentication::UserId;
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
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    // New passwords must be greater than 12 and shorter than 129
    if form.new_password.expose_secret().len() < 13 || form.new_password.expose_secret().len() > 128
    {
        FlashMessage::error(
            "New password must be longer than 12 characters and shorter than 129 characters",
        )
        .send();
        return Ok(utils::see_other("/admin/password"));
    }

    // New passwords must match
    if form.new_password.expose_secret() != form.new_password_confirm.expose_secret() {
        let flash_message_text = "You entered two different new passwords - \
                    the field values must match.";
        FlashMessage::error(flash_message_text).send();
        return Ok(utils::see_other("/admin/password"));
    }
    // Entered current password must be correct
    let username = admin::fetch_username(&user_id.0, &db_pool)
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
    authentication::change_password(user_id.0, form.0.new_password, &db_pool)
        .await
        .map_err(utils::error_500)?;

    FlashMessage::info("You have successfully changed your password").send();
    Ok(utils::see_other("/admin/password"))
}
