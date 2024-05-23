use crate::idempotency::IdempotencyKey;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::{Executor, PgPool, Postgres, Transaction};

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: uuid::Uuid,
    http_response: HttpResponse,
) -> Result<HttpResponse, anyhow::Error> {
    let (response_head, body) = http_response.into_parts();
    let status_code = response_head.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(response_head.headers().len());
        for (name, value) in response_head.headers() {
            let name = name.to_string();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value })
        }
        h
    };
    let body = actix_web::body::to_bytes(body)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Unchecked query because response_headers is our own
    // custom-defined type which sqlx doesn't handle
    transaction
        .execute(sqlx::query_unchecked!(
            r#"
        UPDATE idempotency SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
            user_id,
            idempotency_key.as_ref(),
            status_code,
            headers,
            body.as_ref()
        ))
        .await?;
    transaction.commit().await?;

    let http_response = response_head.set_body(body).map_into_boxed_body();
    Ok(http_response)
}

pub async fn try_processing(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: uuid::Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;
    let query = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref(),
    );
    let num_inserted_rows = transaction.execute(query).await?.rows_affected();

    if num_inserted_rows == 0 {
        let saved_response = get_saved_response(db_pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it"))?;

        Ok(NextAction::ReturnSavedResponse(saved_response))
    } else {
        Ok(NextAction::StartProcessing(transaction))
    }
}

async fn get_saved_response(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: uuid::Uuid,
) -> Result<Option<HttpResponse>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT
            response_status_code AS "response_status_code!",
            response_headers AS "response_headers!: Vec<HeaderPairRecord>",
            response_body AS "response_body!"
        FROM idempotency
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(row) = saved_response {
        let status_code = StatusCode::from_u16(row.response_status_code.try_into()?)?;
        let mut response = HttpResponse::build(status_code);
        for header in row.response_headers {
            response.append_header((header.name, header.value));
        }
        Ok(Some(response.body(row.response_body)))
    } else {
        Ok(None)
    }
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(HttpResponse),
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_header_pair")
    }
}
