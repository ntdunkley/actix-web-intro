use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Duration;
use tracing::field::display;
use tracing::Span;
use uuid::Uuid;

type PgTransaction = Transaction<'static, Postgres>;

pub async fn run_worker_until_stopped(
    config: crate::config::Settings,
) -> Result<(), anyhow::Error> {
    let db_pool = crate::startup::get_db_pool(&config.database);
    let email_client = config.email_client.client();

    worker_loop(db_pool, email_client).await
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

async fn worker_loop(db_pool: PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&db_pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Ok(ExecutionOutcome::TaskCompleted) => {}
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty
    ),
    err
)]
pub async fn try_execute_task(
    db_pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    if let Some((transaction, issue_id, email)) = dequeue_task(db_pool).await? {
        Span::current()
            .record("newsletter_issue_id", &display(issue_id))
            .record("subscriber_email", &display(&email));

        // Send email
        match SubscriberEmail::parse(email.clone()) {
            Ok(email) => {
                let issue = get_issue(db_pool, issue_id).await?;
                if let Err(e) = email_client
                    .send_email(
                        &email,
                        &issue.title,
                        &issue.html_content,
                        &issue.text_content,
                    )
                    .await
                {
                    tracing::error!(
                        error.cause_chain = ?e,
                        error.message = %e,
                        "Failed to deliver issue to a confirmed subscriber. Skipping.",
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
        delete_task(transaction, issue_id, &email).await?;
        Ok(ExecutionOutcome::TaskCompleted)
    } else {
        Ok(ExecutionOutcome::EmptyQueue)
    }
}

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db_pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;

    let query = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#
    );
    let row = query.fetch_optional(&mut *transaction).await?;
    if let Some(row) = row {
        Ok(Some((
            transaction,
            row.newsletter_issue_id,
            row.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    subscriber_email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND
            subscriber_email = $2
        "#,
        issue_id,
        subscriber_email
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(db_pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issue
        WHERE
        newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(db_pool)
    .await?;
    Ok(issue)
}
