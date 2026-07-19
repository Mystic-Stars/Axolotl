use crate::api::Result;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use tauri::plugin::TauriPlugin;
use tauri::{Manager, Runtime, UserAttentionType};
use theseus::prelude::*;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("auth")
        .invoke_handler(tauri::generate_handler![
            check_reachable,
            login,
            begin_yggdrasil_login,
            finish_yggdrasil_login,
            list_yggdrasil_saved_logins,
            get_yggdrasil_password,
            set_yggdrasil_password,
            delete_yggdrasil_password,
            add_offline_user,
            remove_user,
            get_default_user,
            set_default_user,
            get_users,
        ])
        .build()
}

/// Checks if the authentication servers are reachable.
#[tauri::command]
pub async fn check_reachable() -> Result<()> {
    minecraft_auth::check_reachable().await?;
    Ok(())
}

/// Authenticate a user with Hydra - part 1
/// This begins the authentication flow quasi-synchronously, returning a URL to visit (that the user will sign in at)
#[tauri::command]
pub async fn login<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<Credentials>> {
    let flow = minecraft_auth::begin_login().await?;

    let start = Utc::now();

    if let Some(window) = app.get_webview_window("signin") {
        window.close()?;
    }

    let window = tauri::WebviewWindowBuilder::new(
        &app,
        "signin",
        tauri::WebviewUrl::External(flow.auth_request_uri.parse().map_err(
            |_| {
                theseus::ErrorKind::OtherError(
                    "Error parsing auth redirect URL".to_string(),
                )
                .as_error()
            },
        )?),
    )
    .title("Sign into Axolotl Launcher")
    .always_on_top(true)
    .center()
    .build()?;

    window.request_user_attention(Some(UserAttentionType::Critical))?;

    while (Utc::now() - start) < Duration::minutes(10) {
        if window.title().is_err() {
            // user closed window, cancelling flow
            return Ok(None);
        }

        if window
            .url()?
            .as_str()
            .starts_with("https://login.live.com/oauth20_desktop.srf")
            && let Some((_, code)) =
                window.url()?.query_pairs().find(|x| x.0 == "code")
        {
            window.close()?;
            let val = minecraft_auth::finish_login(&code.clone(), flow).await?;

            return Ok(Some(val));
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    window.close()?;
    Ok(None)
}

#[tauri::command]
pub async fn begin_yggdrasil_login(
    api_root: String,
    login: String,
    password: String,
) -> Result<minecraft_auth::YggdrasilLoginResult> {
    Ok(
        minecraft_auth::begin_yggdrasil_login(&api_root, &login, &password)
            .await?,
    )
}

#[tauri::command]
pub async fn finish_yggdrasil_login(
    flow_id: uuid::Uuid,
    profile_id: uuid::Uuid,
) -> Result<Credentials> {
    Ok(minecraft_auth::finish_yggdrasil_login(flow_id, profile_id).await?)
}

const YGGDRASIL_SAVED_LOGINS_KEY: &str = "yggdrasil-saved-logins";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SavedYggdrasilLogin {
    pub api_root: String,
    pub login: String,
}

#[tauri::command]
pub fn list_yggdrasil_saved_logins() -> Result<Vec<SavedYggdrasilLogin>> {
    read_yggdrasil_saved_logins()
}

#[tauri::command]
pub fn get_yggdrasil_password(
    api_root: String,
    login: String,
) -> Result<Option<String>> {
    let entry = yggdrasil_password_entry(&api_root, &login)?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(keyring_error(error)),
    }
}

#[tauri::command]
pub fn set_yggdrasil_password(
    api_root: String,
    login: String,
    password: String,
) -> Result<()> {
    if password.is_empty() {
        return delete_yggdrasil_password(api_root, login);
    }
    let saved_login = normalize_saved_yggdrasil_login(&api_root, &login)?;
    yggdrasil_password_entry(&saved_login.api_root, &saved_login.login)?
        .set_password(&password)
        .map_err(keyring_error)?;

    let mut saved_logins = read_yggdrasil_saved_logins()?;
    upsert_saved_yggdrasil_login(&mut saved_logins, saved_login);
    write_yggdrasil_saved_logins(&saved_logins)
}

#[tauri::command]
pub fn delete_yggdrasil_password(
    api_root: String,
    login: String,
) -> Result<()> {
    let saved_login = normalize_saved_yggdrasil_login(&api_root, &login)?;
    match yggdrasil_password_entry(&saved_login.api_root, &saved_login.login)?
        .delete_credential()
    {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(error) => return Err(keyring_error(error)),
    }

    let mut saved_logins = read_yggdrasil_saved_logins()?;
    remove_saved_yggdrasil_login(&mut saved_logins, &saved_login);
    write_yggdrasil_saved_logins(&saved_logins)
}

fn yggdrasil_password_entry(
    api_root: &str,
    login: &str,
) -> Result<keyring::Entry> {
    let login = login.trim();
    if login.is_empty() {
        return Err(theseus::ErrorKind::InputError(
            "The Yggdrasil account name cannot be empty".to_string(),
        )
        .as_error()
        .into());
    }
    let api_root = minecraft_auth::normalize_yggdrasil_api_root(api_root)?;
    keyring::Entry::new(
        theseus::brand::BUNDLE_IDENTIFIER,
        &format!("{api_root}\n{login}"),
    )
    .map_err(keyring_error)
}

fn normalize_saved_yggdrasil_login(
    api_root: &str,
    login: &str,
) -> Result<SavedYggdrasilLogin> {
    let login = login.trim();
    if login.is_empty() {
        return Err(theseus::ErrorKind::InputError(
            "The Yggdrasil account name cannot be empty".to_string(),
        )
        .as_error()
        .into());
    }
    Ok(SavedYggdrasilLogin {
        api_root: minecraft_auth::normalize_yggdrasil_api_root(api_root)?,
        login: login.to_string(),
    })
}

fn yggdrasil_saved_logins_entry() -> Result<keyring::Entry> {
    keyring::Entry::new(
        theseus::brand::BUNDLE_IDENTIFIER,
        YGGDRASIL_SAVED_LOGINS_KEY,
    )
    .map_err(keyring_error)
}

fn read_yggdrasil_saved_logins() -> Result<Vec<SavedYggdrasilLogin>> {
    match yggdrasil_saved_logins_entry()?.get_password() {
        Ok(saved_logins) => match serde_json::from_str(&saved_logins) {
            Ok(saved_logins) => Ok(saved_logins),
            Err(error) => {
                tracing::warn!(
                    "Ignoring an invalid saved Yggdrasil login index: {error}"
                );
                Ok(Vec::new())
            }
        },
        Err(keyring::Error::NoEntry) => Ok(Vec::new()),
        Err(error) => Err(keyring_error(error)),
    }
}

fn write_yggdrasil_saved_logins(
    saved_logins: &[SavedYggdrasilLogin],
) -> Result<()> {
    let entry = yggdrasil_saved_logins_entry()?;
    if saved_logins.is_empty() {
        return match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(keyring_error(error)),
        };
    }

    let saved_logins =
        serde_json::to_string(saved_logins).map_err(|error| {
            theseus::ErrorKind::OtherError(format!(
                "Unable to serialize saved Yggdrasil logins: {error}"
            ))
            .as_error()
        })?;
    entry.set_password(&saved_logins).map_err(keyring_error)
}

fn upsert_saved_yggdrasil_login(
    saved_logins: &mut Vec<SavedYggdrasilLogin>,
    saved_login: SavedYggdrasilLogin,
) {
    saved_logins.retain(|entry| entry != &saved_login);
    saved_logins.push(saved_login);
    saved_logins.sort_by(|left, right| {
        left.login
            .cmp(&right.login)
            .then_with(|| left.api_root.cmp(&right.api_root))
    });
}

fn remove_saved_yggdrasil_login(
    saved_logins: &mut Vec<SavedYggdrasilLogin>,
    saved_login: &SavedYggdrasilLogin,
) {
    saved_logins.retain(|entry| entry != saved_login);
}

fn keyring_error(
    error: keyring::Error,
) -> crate::api::TheseusSerializableError {
    theseus::ErrorKind::OtherError(format!(
        "Unable to access the system credential store: {error}"
    ))
    .as_error()
    .into()
}

#[tauri::command]
pub async fn add_offline_user(username: String) -> Result<Credentials> {
    Ok(minecraft_auth::add_offline_user(&username).await?)
}

#[tauri::command]
pub async fn remove_user(user: uuid::Uuid) -> Result<()> {
    Ok(minecraft_auth::remove_user(user).await?)
}

#[tauri::command]
pub async fn get_default_user(
    offline_mode: bool,
) -> Result<Option<uuid::Uuid>> {
    Ok(minecraft_auth::get_default_user(offline_mode).await?)
}

#[tauri::command]
pub async fn set_default_user(user: uuid::Uuid) -> Result<()> {
    Ok(minecraft_auth::set_default_user(user).await?)
}

/// Get a copy of the list of all user credentials
#[tauri::command]
pub async fn get_users(
    offline_mode: bool,
) -> Result<Vec<minecraft_auth::MinecraftUser>> {
    Ok(minecraft_auth::users(offline_mode).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved_login(api_root: &str, login: &str) -> SavedYggdrasilLogin {
        SavedYggdrasilLogin {
            api_root: api_root.to_string(),
            login: login.to_string(),
        }
    }

    #[test]
    fn saved_login_index_upserts_and_sorts_entries() {
        let mut saved_logins =
            vec![saved_login("https://example.com", "second")];
        upsert_saved_yggdrasil_login(
            &mut saved_logins,
            saved_login("https://example.com", "first"),
        );
        upsert_saved_yggdrasil_login(
            &mut saved_logins,
            saved_login("https://example.com", "second"),
        );

        assert_eq!(saved_logins.len(), 2);
        assert_eq!(saved_logins[0].login, "first");
        assert_eq!(saved_logins[1].login, "second");
    }

    #[test]
    fn saved_login_index_removes_only_matching_entry() {
        let removed =
            saved_login("https://first.example", "player@example.com");
        let retained =
            saved_login("https://second.example", "player@example.com");
        let mut saved_logins = vec![removed.clone(), retained.clone()];

        remove_saved_yggdrasil_login(&mut saved_logins, &removed);

        assert_eq!(saved_logins, vec![retained]);
    }
}
