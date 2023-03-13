use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Params {
    subscription_token: String,
}

#[tracing::instrument("Confirming a pending subscriber", skip_all)]
pub async fn confirm(params: web::Query<Params>, db_pool: web::Data<PgPool>) -> HttpResponse {
    let subscriber_id =
        match get_subscriber_id_from_token(&db_pool, &params.subscription_token).await {
            Ok(id) => id,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    match subscriber_id {
        None => HttpResponse::Unauthorized().finish(),
        Some(id) => {
            if confirm_subscriber(&db_pool, id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;
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
