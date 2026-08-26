//! Persistence for the user-configurable proxy settings.

use sqlx::{Executor, Row, Sqlite};

use crate::util::proxy::{ProxyConfig, ProxyMode};

const PROXY_COLUMNS_QUERY: &str = "\
    SELECT proxy_mode, proxy_url, proxy_username, proxy_password \
    FROM settings WHERE id = 0";

pub async fn get<'a, E>(exec: E) -> crate::Result<ProxyConfig>
where
    E: Executor<'a, Database = Sqlite> + Copy,
{
    let row = sqlx::query(PROXY_COLUMNS_QUERY).fetch_one(exec).await?;
    Ok(ProxyConfig {
        mode: ProxyMode::from_string(&row.get::<String, _>("proxy_mode")),
        url: row.get::<String, _>("proxy_url"),
        username: row.get::<String, _>("proxy_username"),
        password: row.get::<String, _>("proxy_password"),
    })
}

pub async fn set<'a, E>(exec: E, config: &ProxyConfig) -> crate::Result<()>
where
    E: Executor<'a, Database = Sqlite> + Copy,
{
    config.validate()?;
    sqlx::query(
        "UPDATE settings \
         SET proxy_mode = ?, proxy_url = ?, proxy_username = ?, proxy_password = ? \
         WHERE id = 0",
    )
    .bind(config.mode.as_str())
    .bind(config.url.trim())
    .bind(config.username.trim())
    .bind(config.password.clone())
    .execute(exec)
    .await?;
    Ok(())
}
