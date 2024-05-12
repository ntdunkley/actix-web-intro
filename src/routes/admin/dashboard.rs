use crate::authentication::UserId;
use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::utils;

pub async fn admin_dashboard(
    db_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = fetch_username(&user_id.0, &db_pool)
        .await
        .map_err(utils::error_500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
                <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin dashboard</title>
                </head>
                <body>
                <p>Welcome {username}!</p>
                <p>Available actions:</p>
                <ol>
                    <li><a href="/admin/password">Change password</a></li>
                    <li><a href="/admin/newsletters">Send a newsletter</a></li>
                    <li>
                        <form name="logoutForm" action="/admin/logout" method="post">
                            <input type="submit" value="Logout">
                        </form>
                    </li>
                </ol>
                </body>
            </html>
            "#
        )))
}

#[tracing::instrument(name = "Fetch username", skip(db_pool))]
pub async fn fetch_username(user_id: &Uuid, db_pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform a query to fetch username")?;

    Ok(row.username)
}
