//! Authentication flow interface

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Serialize;
use std::time::Duration;

use crate::State;
pub use crate::state::YggdrasilLoginResult;
use crate::state::{
    Credentials, MinecraftAccountType, MinecraftLoginFlow, MinecraftProfile,
    YggdrasilAccount,
};
use crate::util::fetch::INSECURE_REQWEST_CLIENT;

#[tracing::instrument]
pub async fn check_reachable() -> crate::Result<()> {
    let resp = INSECURE_REQWEST_CLIENT
        .get("https://sessionserver.mojang.com/session/minecraft/hasJoined")
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    if resp.status() == StatusCode::NO_CONTENT {
        return Ok(());
    }
    resp.error_for_status()?;
    Ok(())
}

#[tracing::instrument]
pub async fn begin_login() -> crate::Result<MinecraftLoginFlow> {
    let state = State::get().await?;

    crate::state::login_begin(&state.pool).await
}

#[tracing::instrument]
pub async fn finish_login(
    code: &str,
    flow: MinecraftLoginFlow,
) -> crate::Result<Credentials> {
    let state = State::get().await?;

    crate::state::login_finish(code, flow, &state.pool).await
}

#[tracing::instrument]
pub async fn add_offline_user(username: &str) -> crate::Result<Credentials> {
    let state = State::get().await?;
    let credentials = Credentials::offline(username)?;
    credentials.upsert(&state.pool).await?;
    Ok(credentials)
}

#[tracing::instrument(skip(password))]
pub async fn begin_yggdrasil_login(
    api_root: &str,
    login: &str,
    password: &str,
) -> crate::Result<YggdrasilLoginResult> {
    let state = State::get().await?;
    crate::state::begin_yggdrasil_login(api_root, login, password, &state.pool)
        .await
}

#[tracing::instrument]
pub async fn finish_yggdrasil_login(
    flow_id: uuid::Uuid,
    profile_id: uuid::Uuid,
) -> crate::Result<Credentials> {
    let state = State::get().await?;
    crate::state::finish_yggdrasil_login(flow_id, profile_id, &state.pool).await
}

pub fn normalize_yggdrasil_api_root(api_root: &str) -> crate::Result<String> {
    crate::state::normalize_api_root(api_root)
}

#[tracing::instrument]
pub async fn get_default_user(
    offline_mode: bool,
) -> crate::Result<Option<uuid::Uuid>> {
    let state = State::get().await?;
    let user = if offline_mode {
        Credentials::get_offline_credential(&state.pool).await?
    } else {
        Credentials::get_active(&state.pool).await?
    };
    Ok(user.map(|user| user.offline_profile.id))
}

#[tracing::instrument]
pub async fn set_default_user(user: uuid::Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    let users = Credentials::get_all_without_refresh(&state.pool).await?;
    let (_, mut user) = users.remove(&user).ok_or_else(|| {
        crate::ErrorKind::OtherError(format!(
            "Tried to get nonexistent user with ID {user}"
        ))
        .as_error()
    })?;

    user.active = true;
    user.upsert(&state.pool).await?;

    Ok(())
}

/// Remove a user account from the database
#[tracing::instrument]
pub async fn remove_user(uuid: uuid::Uuid) -> crate::Result<()> {
    let state = State::get().await?;

    let users = Credentials::get_all_without_refresh(&state.pool).await?;

    if let Some((uuid, user)) = users.remove(&uuid) {
        Credentials::remove(uuid, &state.pool).await?;

        if user.active
            && let Some((_, mut user)) = users.into_iter().next()
        {
            user.active = true;
            user.upsert(&state.pool).await?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
pub struct MinecraftUser {
    pub profile: MinecraftProfile,
    pub account_type: MinecraftAccountType,
    pub access_token: String,
    pub refresh_token: String,
    pub expires: DateTime<Utc>,
    pub active: bool,
    pub yggdrasil: Option<YggdrasilAccount>,
}

impl MinecraftUser {
    async fn from_credentials(credentials: Credentials) -> Self {
        let profile = (*credentials.maybe_online_profile().await).clone();
        Self {
            profile,
            account_type: credentials.account_type,
            access_token: credentials.access_token,
            refresh_token: credentials.refresh_token,
            expires: credentials.expires,
            active: credentials.active,
            yggdrasil: credentials.yggdrasil,
        }
    }
}

/// Get a copy of the list of all user credentials with profile data ready for
/// serialization.
#[tracing::instrument]
pub async fn users(offline_mode: bool) -> crate::Result<Vec<MinecraftUser>> {
    let state = State::get().await?;
    let users = if offline_mode {
        Credentials::get_all_without_refresh(&state.pool).await?
    } else {
        Credentials::get_all(&state.pool).await?
    };
    let credentials = users
        .into_iter()
        .map(|x| x.1)
        .filter(|credentials| !offline_mode || credentials.is_offline());
    let mut hydrated_users = Vec::new();
    for credentials in credentials {
        hydrated_users.push(MinecraftUser::from_credentials(credentials).await);
    }
    Ok(hydrated_users)
}
