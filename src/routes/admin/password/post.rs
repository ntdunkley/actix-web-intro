use actix_web::{web, HttpResponse};
use secrecy::Secret;
use serde::Deserialize;

use crate::session_state::TypedSession;
use crate::utils;

#[derive(Deserialize)]
pub struct FormData {
    _old_password: Secret<String>,
    _new_password: Secret<String>,
    _new_password_confirm: Secret<String>,
}

pub async fn change_password(
    _form: web::Form<FormData>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(utils::error_500)?.is_none() {
        return Ok(utils::see_other("/login"));
    }
    todo!()
}
