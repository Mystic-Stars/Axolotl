//! Authentication flow interface

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

use crate::State;
pub use crate::state::YggdrasilLoginResult;
use crate::state::{
    Credentials, MinecraftAccountType, MinecraftLoginFlow, MinecraftProfile,
    YggdrasilAccount,
};
pub use crate::state::{MinecraftDeviceLoginFlow, MinecraftDeviceLoginPoll};
use crate::util::fetch::INSECURE_REQWEST_CLIENT;
use crate::util::mojang::{mojang_service_url, should_use_mojang_mirror};

#[tracing::instrument]
pub async fn check_reachable() -> crate::Result<()> {
    let url = mojang_service_url(
        "https://sessionserver.mojang.com/session/minecraft/hasJoined",
        should_use_mojang_mirror(),
    );
    let resp = INSECURE_REQWEST_CLIENT
        .get(url.as_ref())
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    if resp.status() == StatusCode::NO_CONTENT {
        return Ok(());
    }
    resp.error_for_status()?;
    Ok(())
}

pub async fn set_mojang_auth_use_mirror(
    use_mirror: bool,
    automatic: bool,
) -> crate::Result<()> {
    let state = State::get().await?;
    state.set_mojang_auth_use_mirror(use_mirror);
    if use_mirror && automatic {
        tracing::info!(
            "Mojang services are unreachable; routing Mojang service requests through the Fallen proxy"
        );
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct MojangServiceStatus {
    pub service: &'static str,
    pub url: &'static str,
    pub reachable: bool,
}

const MOJANG_SERVICES: [(&str, &str); 5] = [
    ("auth", "https://authserver.mojang.com/"),
    ("account", "https://api.mojang.com/"),
    (
        "session",
        "https://sessionserver.mojang.com/session/minecraft/hasJoined",
    ),
    ("services", "https://api.minecraftservices.com/"),
    (
        "profiles",
        "https://api.mojang.com/users/profiles/minecraft/",
    ),
];

#[tracing::instrument]
pub async fn check_mojang_services() -> Vec<MojangServiceStatus> {
    futures::future::join_all(MOJANG_SERVICES.map(
        |(service, url)| async move {
            let reachable = INSECURE_REQWEST_CLIENT
                .get(url)
                .timeout(Duration::from_secs(5))
                .send()
                .await
                .is_ok();
            MojangServiceStatus {
                service,
                url,
                reachable,
            }
        },
    ))
    .await
}

#[tracing::instrument]
pub async fn begin_login() -> crate::Result<MinecraftLoginFlow> {
    let state = State::get().await?;

    crate::state::login_begin(&state.pool).await
}

#[tracing::instrument]
pub async fn begin_browser_login() -> crate::Result<MinecraftLoginFlow> {
    let state = State::get().await?;
    crate::state::browser_login_begin(&state.pool).await
}

#[tracing::instrument]
pub async fn begin_device_login() -> crate::Result<MinecraftDeviceLoginFlow> {
    crate::state::device_login_begin().await
}

#[tracing::instrument]
pub async fn poll_device_login(
    device_code: &str,
) -> crate::Result<MinecraftDeviceLoginPoll> {
    let state = State::get().await?;
    crate::state::device_login_poll(device_code, &state.pool).await
}

#[tracing::instrument]
pub async fn finish_login(
    code: &str,
    state: &str,
    flow: MinecraftLoginFlow,
) -> crate::Result<Credentials> {
    let app_state = State::get().await?;

    crate::state::login_finish(code, state, flow, &app_state.pool).await
}

#[tracing::instrument]
pub async fn add_offline_user(
    username: &str,
    uuid: Option<Uuid>,
) -> crate::Result<Credentials> {
    let state = State::get().await?;
    let credentials = match uuid {
        Some(uuid) => Credentials::offline_with_uuid(username, uuid)?,
        None => Credentials::offline(username)?,
    };

    if uuid.is_some() {
        let users = Credentials::get_all_without_refresh(&state.pool).await?;
        if users
            .iter()
            .any(|user| user.account_id() == credentials.account_id())
        {
            return Err(crate::ErrorKind::InputError(
                "An account with this UUID already exists".to_string(),
            )
            .as_error());
        }
    }

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
) -> crate::Result<Option<String>> {
    let state = State::get().await?;
    let user = if offline_mode {
        Credentials::get_offline_credential(&state.pool).await?
    } else {
        Credentials::get_active(&state.pool).await?
    };
    Ok(user.map(|user| user.account_id()))
}

#[tracing::instrument]
pub async fn set_default_user(account_id: &str) -> crate::Result<()> {
    let state = State::get().await?;
    let users = Credentials::get_all_without_refresh(&state.pool).await?;
    let mut user = users
        .into_iter()
        .find(|user| user.account_id() == account_id)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Tried to get nonexistent user with ID {account_id}"
            ))
            .as_error()
        })?;

    user.active = true;
    user.upsert(&state.pool).await?;

    Ok(())
}

/// Remove a user account from the database
#[tracing::instrument]
pub async fn remove_user(account_id: &str) -> crate::Result<()> {
    let state = State::get().await?;

    let mut users = Credentials::get_all_without_refresh(&state.pool).await?;

    if let Some(index) = users
        .iter()
        .position(|user| user.account_id() == account_id)
    {
        let user = users.remove(index);
        Credentials::remove(account_id, &state.pool).await?;

        if user.active
            && let Some(mut user) = users.into_iter().next()
        {
            user.active = true;
            user.upsert(&state.pool).await?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
pub struct MinecraftUser {
    pub account_id: String,
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
            account_id: credentials.account_id(),
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
        .filter(|credentials| !offline_mode || credentials.is_offline());
    let mut hydrated_users = Vec::new();
    for credentials in credentials {
        hydrated_users.push(MinecraftUser::from_credentials(credentials).await);
    }
    Ok(hydrated_users)
}
