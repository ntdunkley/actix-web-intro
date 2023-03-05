use actix_web::{web, HttpResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    _subscription_token: String,
}

#[tracing::instrument("Confirming a pending subscriber", skip_all)]
pub async fn confirm_subscriber(_params: web::Query<Params>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
