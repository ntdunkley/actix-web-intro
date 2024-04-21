use anyhow::{anyhow, Context};
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, db_pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    db_pool: &PgPool,
) -> Result<Uuid, AuthError> {
    // We want to verify a hash even if the user doesn't exist. This is to help prevent timing
    // attacks (the difference in response times when a user exists and when it doesn't).
    // Therefore we set a default hash.
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
                gZiV/M1gPc22ElAH/Jh1Hw$\
                CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, db_pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    crate::telemetry::spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn task to verify password hash")??;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow!("Unknown username")))
}

#[tracing::instrument(name = "Change password", skip(new_password, db_pool))]
pub async fn change_password(
    user_id: Uuid,
    new_password: Secret<String>,
    db_pool: &PgPool,
) -> Result<(), anyhow::Error> {
    // Compute password hash
    let password_hash =
        crate::telemetry::spawn_blocking_with_tracing(move || compute_password_hash(new_password))
            .await?
            .context("Failed to hash password")?;

    // Change password
    sqlx::query!(
        r#"
        UPDATE users SET
            password = $2
        WHERE user_id = $1
        "#,
        user_id,
        password_hash.expose_secret()
    )
    .execute(db_pool)
    .await
    .context("Failed to change user's password in database")?;

    Ok(())
}

#[tracing::instrument(name = "Get stored credentials", skip(username, db_pool))]
async fn get_stored_credentials(
    username: &str,
    db_pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        "SELECT user_id, password FROM users WHERE username = $1",
        username
    )
    .fetch_optional(db_pool)
    .await
    .context("Failed to perform query to retrieve stored credentials")?
    .map(|row| (row.user_id, Secret::new(row.password)));

    Ok(row)
}

#[tracing::instrument(name = "Verify password hash", skip(expected_password_hash, password))]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).expect("Could not create Argon params"),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(Secret::new(password_hash))
}
