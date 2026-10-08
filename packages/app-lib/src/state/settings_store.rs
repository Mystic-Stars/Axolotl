//! Structured text store for the settings Axolotl keeps.
//!
//! A document is authoritative for the keys it carries, so it only starts
//! shadowing the database once a write has created it: an absent document
//! leaves `Settings::get` reading the row exactly as before.
//!
//! `THESEUS_SETTINGS_CONFIG_DIR` points at a read-only directory of documents a
//! deployment ships, which are read before the local ones and the row; only the
//! local directory is ever written.
//!
//! A local document carries what differs from those shipped defaults: a key
//! whose value is what the deployment already provides is left out of it, so
//! that changing that default later still reaches the settings, and a key that
//! returns to its default leaves the document again.
//!
//! Keys are only listed here once every reader goes through `Settings::get`,
//! which is the only place that applies the stored values.
//!
//! Bump `SCHEMA_VERSION` whenever a domain's key set changes, including when
//! a key is dropped: that is what stops a build which still knows the removed
//! key from writing the document again. It versions the shape of a document,
//! unlike the `version` the `state` domain carries, which moves values. A
//! document a newer build wrote is still read, so a build that knows less keeps
//! showing the values the user chose instead of what the row froze.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
#[cfg(test)]
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::state::{DirectoryInfo, Settings};
use crate::util::proxy::ProxyConfig;

const DIR_NAME: &str = "settings";
const SCHEMA_VERSION: u32 = 1;

/// The marker a finished settings handover leaves behind.
const HANDOVER_MARKER_FILE: &str = ".handed-over";

/// How often one start writes the documents of a handover out, before leaving
/// the rest to the next one.
const HANDOVER_ATTEMPTS: usize = 3;

/// The documents the store owns, with the keys each one carries.
const DOMAINS: &[(&str, &[&str])] = &[
    (
        "appearance",
        &[
            "accent_color",
            "advanced_rendering",
            "auto_hide_downloads_button",
            "close_behavior",
            "collapsed_navigation",
            "custom_background_blur",
            "custom_background_component_opacity",
            "custom_background_opacity",
            "custom_background_path",
            "custom_window_title_enabled",
            "default_page",
            "default_window_title",
            "developer_mode",
            "feature_flags",
            "hidden_nav_items",
            "hide_nametag_skins_page",
            "home_layout",
            "home_widget_background_opacity",
            "home_widgets",
            "locale",
            "log_level",
            "minimal_home_instance_id",
            "mono_font",
            "native_decorations",
            "show_files_tab_in_instances",
            "show_screenshots_tab_in_instances",
            "show_skin_selector_in_sidebar",
            "show_worlds_tab_in_instances",
            "sidebar_instance_count",
            "sync_features_across_devices",
            "theme",
            "toggle_sidebar",
            "transparent_background",
            "transparent_background_blur",
            "transparent_background_opacity",
            "ui_font",
        ],
    ),
    (
        "download",
        &[
            "auto_concurrent_downloads",
            "bypass_curseforge_download_restrictions",
            "curseforge_source",
            "doh_enabled",
            "max_concurrent_downloads",
            "max_concurrent_writes",
            "minecraft_file_source",
            "minecraft_metadata_source",
            "modrinth_source",
            "mojang_auth_source",
        ],
    ),
    (
        "game",
        &[
            "auto_set_java_high_performance_mode",
            "custom_env_vars",
            "enter_lightweight_mode_on_game_launch",
            "extra_launch_args",
            "force_fullscreen",
            "game_resolution",
            "hide_on_process_start",
            "hooks",
            "maximize_window",
            "memory",
        ],
    ),
    ("backup", &["backup_repository_path"]),
    ("bootstrap", &["custom_dir", "prev_custom_dir"]),
    (
        "network",
        &[
            "allow_external_scheme",
            "allow_privileged_scheme",
            "ignore_ssl_errors",
            "terracotta_public_nodes",
        ],
    ),
    (
        "privacy",
        &["discord_rpc", "telemetry", "telemetry_consent_version"],
    ),
    (
        "state",
        &[
            "migrated",
            "onboarded",
            "onboarding_instance_tour_completed",
            "onboarding_version",
            "pending_update_toast_for_version",
            "version",
        ],
    ),
];

/// The proxy keeps its own document and keys rather than a domain, because no
/// `Settings` field carries it.
const PROXY_DOCUMENT: &(&str, &[&str]) =
    &("proxy", &["proxy_mode", "proxy_url", "proxy_username"]);

#[derive(Deserialize, Serialize)]
struct Document {
    schema_version: u32,
    written_by: String,
    data: Value,
}

/// A document `Newer` than this build is still read - its values are what the
/// user last chose - but it is never written back, which is what keeps a build
/// that knows less from dropping what a newer one stored.
enum Stored {
    Missing,
    Unreadable,
    Newer(Document),
    Ready(Document),
}

/// The settings directory, resolved before `State` exists.
static SETTINGS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// The read-only directory of documents a deployment ships, when it has one.
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

const CONFIG_DIR_ENV: &str = "THESEUS_SETTINGS_CONFIG_DIR";

/// Serializes the writers, which read a document before writing it back: two
/// settings changing at once would otherwise lose one of them.
static WRITES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
thread_local! {
    static TEST_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static TEST_CONFIG_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub(crate) fn init(app_identifier: &str) {
    let Some(settings_dir) =
        DirectoryInfo::initial_settings_dir_path(app_identifier)
    else {
        return;
    };
    let _ = SETTINGS_DIR.set(settings_dir);
    if let Some(config_dir) = std::env::var_os(CONFIG_DIR_ENV)
        && !config_dir.is_empty()
    {
        let _ = CONFIG_DIR.set(PathBuf::from(config_dir));
    }
}

/// Where the documents live: the directory startup resolved, or one of this
/// thread's own while running tests.
fn settings_root() -> Option<PathBuf> {
    #[cfg(test)]
    return TEST_DIR.with(|dir| {
        let mut dir = dir.borrow_mut();
        let path =
            dir.get_or_insert_with(|| tempfile::tempdir().unwrap().keep());
        Some(path.clone())
    });

    #[cfg(not(test))]
    SETTINGS_DIR.get().cloned()
}

/// Whether the row still has to hand its settings over to the documents, which
/// is what an installation that predates them does once: `database_existed`
/// keeps a fresh installation out, and both the marker a finished handover
/// leaves behind and a complete set of documents keep one that already took the
/// row over from doing it again.
pub(crate) async fn needs_seeding(database_existed: bool) -> bool {
    needs_seeding_at(SETTINGS_DIR.get().map(PathBuf::as_path), database_existed)
        .await
}

async fn needs_seeding_at(
    settings_dir: Option<&Path>,
    database_existed: bool,
) -> bool {
    let Some(settings_dir) = settings_dir else {
        return false;
    };
    if !database_existed {
        return false;
    }
    if tokio::fs::try_exists(handover_marker(settings_dir))
        .await
        .unwrap_or(false)
    {
        return false;
    }
    !documents_exist(settings_dir).await
}

/// Hands the row's settings and proxy over to the documents, and records that
/// once every one of them has taken them. Reports whether the handover is on
/// record: one that fails part way is attempted again, at the next start at the
/// latest, because nothing else says the documents are complete.
pub(crate) async fn hand_over(
    settings: &Settings,
    proxy: &[(&str, Value)],
) -> bool {
    let _writing = WRITES.lock().await;
    for attempt in 1..=HANDOVER_ATTEMPTS {
        if hand_over_to(settings, proxy).await {
            return mark_handed_over().await;
        }
        tracing::warn!(attempt, "Retrying the settings handover");
    }
    false
}

/// Whether every document took over the settings of its domain.
async fn hand_over_to(settings: &Settings, proxy: &[(&str, Value)]) -> bool {
    let mut complete = store_all(settings).await;
    let (name, _) = *PROXY_DOCUMENT;
    let Some(path) = local_path(name) else {
        return false;
    };
    if let Err(error) = set_entries_at(&path, proxy).await {
        tracing::warn!(
            path = %path.display(),
            %error,
            "Failed to save the {name} settings"
        );
        complete = false;
    }
    complete
}

/// Writes the marker a finished handover leaves behind, which only happens once
/// every document is in place.
async fn mark_handed_over() -> bool {
    let Some(path) = settings_root().map(|root| handover_marker(&root)) else {
        return false;
    };
    let written = async {
        if let Some(parent) = path.parent() {
            crate::util::io::create_dir_all(parent).await?;
        }
        crate::util::io::write(&path, env!("CARGO_PKG_VERSION")).await
    };
    match written.await {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "Failed to record the settings handover"
            );
            false
        }
    }
}

fn handover_marker(settings_dir: &Path) -> PathBuf {
    settings_dir.join(DIR_NAME).join(HANDOVER_MARKER_FILE)
}

async fn documents_exist(settings_dir: &Path) -> bool {
    for (name, _) in DOMAINS.iter().copied().chain([*PROXY_DOCUMENT]) {
        if !tokio::fs::try_exists(document_path(settings_dir, name))
            .await
            .unwrap_or(false)
        {
            return false;
        }
    }
    true
}

fn document_path(settings_dir: &Path, name: &str) -> PathBuf {
    settings_dir.join(DIR_NAME).join(format!("{name}.json"))
}

fn local_path(name: &str) -> Option<PathBuf> {
    Some(document_path(&settings_root()?, name))
}

fn config_path(name: &str) -> Option<PathBuf> {
    Some(config_dir()?.join(format!("{name}.json")))
}

/// The directory a deployment's defaults live in: the one the environment
/// named, or the one a test gave this thread.
fn config_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(dir) = TEST_CONFIG_DIR.with(|dir| dir.borrow().clone()) {
        return Some(dir);
    }
    CONFIG_DIR.get().cloned()
}

/// The defaults a deployment ships for the document at `path`, which a write
/// leaves where they are.
async fn shipped_defaults(path: &Path) -> Map<String, Value> {
    match path.file_stem().and_then(|name| name.to_str()) {
        Some(name) => match config_path(name) {
            Some(config) => stored_at(&config).await,
            None => Map::new(),
        },
        None => Map::new(),
    }
}

/// What every key falls back to when no document carries it, which is what a
/// write compares against once a deployment ships nothing for it.
fn settings_defaults() -> Map<String, Value> {
    let Ok(Value::Object(mut defaults)) =
        serde_json::to_value(Settings::default())
    else {
        debug_assert!(false, "the settings default is not an object");
        return Map::new();
    };

    // The proxy owns its keys, so its defaults come from its type rather than
    // from the settings.
    let proxy = ProxyConfig::default();
    defaults.insert("proxy_mode".to_string(), Value::from(proxy.mode.as_str()));
    defaults.insert("proxy_url".to_string(), Value::from(proxy.url));
    defaults.insert("proxy_username".to_string(), Value::from(proxy.username));
    defaults
}

/// Gives `key` to the document, or takes it away when its value is what the
/// build or the deployment already provides: a key that returns to its default
/// leaves the document again, so a changed default still reaches the settings.
fn write_key(
    data: &mut Map<String, Value>,
    key: &str,
    value: Value,
    shipped: &Map<String, Value>,
    defaults: &Map<String, Value>,
) {
    let provided = provided(shipped.get(key), defaults, key);
    if provided == Some(&value) {
        data.remove(key);
    } else {
        data.insert(key.to_string(), value);
    }
}

/// The settings the documents describe: what a build carries by default, then
/// the documents a deployment ships, then the local ones. The database is not
/// part of this, since startup hands it over to the documents once.
pub(crate) async fn read_settings() -> Settings {
    let mut settings = Settings::default();
    let defaults = settings_defaults();
    let pruning = config_dir().is_some();
    for (name, keys) in DOMAINS {
        let shipped = match config_path(name) {
            Some(path) => readable(&path).await,
            None => None,
        };
        let local = match local_path(name) {
            Some(path) => readable(&path).await,
            None => None,
        };

        for document in [&shipped, &local].into_iter().flatten() {
            settings = overlay_document(document, keys, settings);
        }
        if pruning
            && let (Some(path), Some(document)) =
                (local_path(name), local.as_ref())
        {
            prune_repeated(&path, keys, shipped.as_ref(), document, &defaults)
                .await;
        }
    }
    if pruning {
        prune_on_read(*PROXY_DOCUMENT, &defaults).await;
    }
    settings
}

/// Prunes a document the read chain does not overlay, so a key it repeats
/// stops shadowing a later change of the value it repeats.
async fn prune_on_read(
    (name, keys): (&str, &[&str]),
    defaults: &Map<String, Value>,
) {
    let shipped = match config_path(name) {
        Some(path) => readable(&path).await,
        None => None,
    };
    let Some(path) = local_path(name) else {
        return;
    };
    let Some(document) = readable(&path).await else {
        return;
    };
    prune_repeated(&path, keys, shipped.as_ref(), &document, defaults).await;
}

/// Drops what a local document repeats of a value the build or the deployment
/// already provides. Writing a value into a deployment's own document is how
/// its owner says a key should follow it, so the local copy stops shadowing it
/// as soon as the two agree, and a later change to that default still arrives.
async fn prune_repeated(
    path: &Path,
    keys: &[&str],
    shipped: Option<&Document>,
    local: &Document,
    defaults: &Map<String, Value>,
) {
    let Some(stored) = local.data.as_object() else {
        return;
    };
    let shipped_entries =
        shipped.and_then(|document| document.data.as_object());
    let repeated = keys.iter().any(|key| {
        stored.get(*key).is_some_and(|value| {
            let provided = provided(
                shipped_entries.and_then(|entries| entries.get(*key)),
                defaults,
                key,
            );
            provided == Some(value)
        })
    });
    if !repeated {
        return;
    }

    let _writing = WRITES.lock().await;
    let shipped = shipped_entries.cloned().unwrap_or_default();
    if let Err(error) = prune_at(path, keys, &shipped, defaults).await {
        tracing::warn!(
            path = %path.display(),
            %error,
            "Failed to drop the entries a default already provides"
        );
    }
}

/// What `key` resolves to when no document carries it: the deployment's value
/// when it ships one, and the build's default otherwise.
fn provided<'a>(
    shipped: Option<&'a Value>,
    defaults: &'a Map<String, Value>,
    key: &str,
) -> Option<&'a Value> {
    shipped.or_else(|| defaults.get(key))
}

/// Layers the documents of one domain, so a default gives way to the local file
/// and to the ones a deployment ships.
async fn overlay_layers(
    defaults: Option<&Path>,
    local: Option<&Path>,
    keys: &[&str],
    settings: Settings,
) -> Settings {
    let mut settings = settings;
    for path in [defaults, local].into_iter().flatten() {
        settings = overlay_from(path, keys, settings).await;
    }
    settings
}

/// Persists every domain, without letting a failure reach the caller: the row
/// keeps the values a document has not taken over yet.
pub(crate) async fn store(settings: &Settings) {
    let _writing = WRITES.lock().await;
    store_all(settings).await;
}

/// Persists every domain and reports whether all of them took the settings,
/// which is what a handover retries on.
async fn store_all(settings: &Settings) -> bool {
    let mut complete = true;
    for (name, keys) in DOMAINS {
        let Some(path) = local_path(name) else {
            return false;
        };
        if let Err(error) = store_to(&path, keys, settings).await {
            tracing::warn!(
                path = %path.display(),
                %error,
                "Failed to save the {name} settings"
            );
            complete = false;
        }
    }
    complete
}

/// Drops the keys no domain knows, for documents another version left behind.
/// Documents written by a newer build are left alone.
pub async fn sanitise() -> crate::Result<usize> {
    let _writing = WRITES.lock().await;
    let mut removed = 0;
    for (name, keys) in DOMAINS {
        let Some(path) = local_path(name) else {
            break;
        };
        removed += sanitise_at(&path, keys).await?;
    }
    Ok(removed)
}

/// Drops the entries that only repeat what the build or a deployment already
/// provides, which is what a document written before those defaults existed
/// carries. Startup runs it once, so a default a deployment changed while the
/// launcher was closed reaches the settings again.
pub(crate) async fn prune_redundant() {
    let defaults = settings_defaults();
    let domains = DOMAINS.iter().copied().chain([*PROXY_DOCUMENT]);
    for (name, keys) in domains {
        let Some(path) = local_path(name) else {
            return;
        };
        let shipped = match config_path(name) {
            Some(config) => readable(&config).await,
            None => None,
        };
        let Some(local) = readable(&path).await else {
            continue;
        };
        prune_repeated(&path, keys, shipped.as_ref(), &local, &defaults).await;
    }
}

async fn prune_at(
    path: &Path,
    keys: &[&str],
    shipped: &Map<String, Value>,
    defaults: &Map<String, Value>,
) -> crate::Result<()> {
    let Stored::Ready(document) = read(path).await else {
        return Ok(());
    };
    let Some(mut data) = document.data.as_object().cloned() else {
        return Ok(());
    };
    let before = data.len();
    for key in keys {
        let Some(value) = data.get(*key).cloned() else {
            continue;
        };
        let provided = provided(shipped.get(*key), defaults, key);
        if provided == Some(&value) {
            data.remove(*key);
        }
    }
    if data.len() == before {
        return Ok(());
    }

    tracing::info!(
        path = %path.display(),
        removed = before - data.len(),
        "Dropped settings entries a default already provides"
    );
    write_document(path, data).await
}

async fn sanitise_at(path: &Path, keys: &[&str]) -> crate::Result<usize> {
    let Stored::Ready(mut document) = read(path).await else {
        return Ok(0);
    };
    let Some(data) = document.data.as_object_mut() else {
        return Ok(0);
    };
    let before = data.len();
    data.retain(|key, _| keys.contains(&key.as_str()));
    let removed = before - data.len();
    if removed == 0 {
        return Ok(0);
    }

    crate::util::io::write(path, serde_json::to_vec_pretty(&document)?).await?;
    tracing::info!(
        path = %path.display(),
        removed,
        "Removed settings keys no domain knows"
    );
    Ok(removed)
}

/// The stored entries of one document, for callers that read a few keys
/// without going through `Settings`. Empty when there is nothing to read.
pub(crate) async fn stored(domain: &str) -> Map<String, Value> {
    stored_layers(
        config_path(domain).as_deref(),
        local_path(domain).as_deref(),
    )
    .await
}

/// Layers the entries of one domain, so a default gives way to the local file.
async fn stored_layers(
    defaults: Option<&Path>,
    local: Option<&Path>,
) -> Map<String, Value> {
    let mut entries = Map::new();
    for path in [defaults, local].into_iter().flatten() {
        entries.extend(stored_at(path).await);
    }
    entries
}

async fn stored_at(path: &Path) -> Map<String, Value> {
    match read(path).await {
        Stored::Ready(document) | Stored::Newer(document) => {
            document.data.as_object().cloned().unwrap_or_default()
        }
        _ => Map::new(),
    }
}

async fn read(path: &Path) -> Stored {
    let contents = match tokio::fs::read(path).await {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Stored::Missing;
        }
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "Could not read a settings document"
            );
            return Stored::Unreadable;
        }
    };
    if contents.iter().all(u8::is_ascii_whitespace) {
        return Stored::Missing;
    }
    let Ok(document) = serde_json::from_slice::<Document>(&contents) else {
        return Stored::Unreadable;
    };
    if !document.data.is_object() {
        return Stored::Unreadable;
    }
    if document.schema_version > SCHEMA_VERSION {
        Stored::Newer(document)
    } else {
        Stored::Ready(document)
    }
}

/// The document at `path` as a reader sees it: a missing or unreadable one has
/// nothing to apply, and one a newer build wrote is read like any other.
async fn readable(path: &Path) -> Option<Document> {
    match read(path).await {
        Stored::Ready(document) => Some(document),
        Stored::Missing => None,
        Stored::Unreadable => {
            tracing::warn!(
                path = %path.display(),
                "Ignoring a settings document that cannot be read"
            );
            None
        }
        Stored::Newer(document) => {
            tracing::debug!(
                path = %path.display(),
                version = document.schema_version,
                "Reading a settings document a newer build wrote"
            );
            Some(document)
        }
    }
}

async fn overlay_from(
    path: &Path,
    keys: &[&str],
    settings: Settings,
) -> Settings {
    match readable(path).await {
        Some(document) => overlay_document(&document, keys, settings),
        None => settings,
    }
}

/// Applies the keys `document` carries on top of `settings`. A single entry this
/// build cannot read is dropped on its own, so one unusable value cannot take
/// the rest of its document with it.
fn overlay_document(
    document: &Document,
    keys: &[&str],
    settings: Settings,
) -> Settings {
    let Some(stored) = document.data.as_object() else {
        return settings;
    };
    let Ok(Value::Object(base)) = serde_json::to_value(&settings) else {
        return settings;
    };
    let entries: Vec<(&String, &Value)> = stored
        .iter()
        .filter(|(key, _)| keys.contains(&key.as_str()))
        .collect();
    let with_entries = |applied: &[(&String, &Value)]| {
        let mut data = base.clone();
        for (key, value) in applied {
            data.insert((*key).clone(), (*value).clone());
        }
        serde_json::from_value::<Settings>(Value::Object(data))
    };

    if let Ok(settings) = with_entries(&entries) {
        return settings;
    }
    let mut applied: Vec<(&String, &Value)> = Vec::new();
    for entry in &entries {
        applied.push(*entry);
        if with_entries(&applied).is_err() {
            applied.pop();
            tracing::warn!(
                key = %entry.0,
                "Ignoring a settings entry this build cannot read"
            );
        }
    }
    match with_entries(&applied) {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(
                %error,
                "Keeping the defaults of a settings document this build cannot read"
            );
            settings
        }
    }
}

async fn store_to(
    path: &Path,
    keys: &[&str],
    settings: &Settings,
) -> crate::Result<()> {
    let Some(mut data) = base_data(path).await else {
        return Ok(());
    };
    let defaults = shipped_defaults(path).await;
    let settings_defaults = settings_defaults();

    let normalized = settings.normalized();
    let mut value = serde_json::to_value(&normalized)?;
    let Some(fields) = value.as_object_mut() else {
        return Ok(());
    };
    fields.retain(|key, _| keys.contains(&key.as_str()));

    for (key, value) in fields.iter() {
        write_key(&mut data, key, value.clone(), &defaults, &settings_defaults);
    }

    write_document(path, data).await
}

/// Writes one key of the domain that lists it, leaving the rest of the
/// document as it is. Keys no domain lists are ignored.
pub(crate) async fn store_key(key: &str, value: Value) {
    let Some(path) = key_path(key) else {
        return;
    };
    let _writing = WRITES.lock().await;
    if let Err(error) = set_key_at(&path, key, value).await {
        tracing::warn!(
            path = %path.display(),
            %error,
            "Failed to save a setting"
        );
    }
}

/// Writes entries of a document the domain table does not derive, which is how
/// a domain that owns its keys is stored. Such a document is left out of
/// `sanitise`, since no domain lists its keys.
pub(crate) async fn store_in(domain: &str, entries: &[(&str, Value)]) {
    let Some(path) = local_path(domain) else {
        return;
    };
    let _writing = WRITES.lock().await;
    if let Err(error) = set_entries_at(&path, entries).await {
        tracing::warn!(
            path = %path.display(),
            %error,
            "Failed to save the {domain} settings"
        );
    }
}

/// Clears `key` while the stored document still holds `value`, which keeps the
/// settings from referring to something that no longer exists.
pub(crate) async fn clear_key_if(key: &str, value: &str) {
    let Some(path) = key_path(key) else {
        return;
    };
    let _writing = WRITES.lock().await;
    if let Err(error) = clear_key_if_at(&path, key, value).await {
        tracing::warn!(
            path = %path.display(),
            %error,
            "Failed to clear a setting"
        );
    }
}

async fn clear_key_if_at(
    path: &Path,
    key: &str,
    value: &str,
) -> crate::Result<()> {
    if stored_at(path).await.get(key).and_then(Value::as_str) != Some(value) {
        return Ok(());
    }
    set_key_at(path, key, Value::Null).await
}

/// The document that lists `key`, for the writers that own a single key.
fn key_path(key: &str) -> Option<PathBuf> {
    let Some((name, _)) = DOMAINS.iter().find(|(_, keys)| keys.contains(&key))
    else {
        tracing::debug!(key, "Ignoring a settings key no domain lists");
        return None;
    };
    local_path(name)
}

async fn set_key_at(path: &Path, key: &str, value: Value) -> crate::Result<()> {
    let Some(mut data) = base_data(path).await else {
        return Ok(());
    };
    let defaults = shipped_defaults(path).await;
    let settings_defaults = settings_defaults();
    write_key(&mut data, key, value, &defaults, &settings_defaults);
    write_document(path, data).await
}

async fn set_entries_at(
    path: &Path,
    entries: &[(&str, Value)],
) -> crate::Result<()> {
    let Some(mut data) = base_data(path).await else {
        return Ok(());
    };
    let defaults = shipped_defaults(path).await;
    let settings_defaults = settings_defaults();
    for (key, value) in entries {
        write_key(&mut data, key, value.clone(), &defaults, &settings_defaults);
    }
    write_document(path, data).await
}

/// The stored entries as a base, so a document another version wrote keeps the
/// entries this build does not know. `None` means the document must not be
/// written, which keeps one from a newer build read-only and leaves one that
/// cannot be read for its owner to sort out.
async fn base_data(path: &Path) -> Option<Map<String, Value>> {
    match read(path).await {
        Stored::Ready(document) => document.data.as_object().cloned(),
        Stored::Newer(document) => {
            tracing::warn!(
                path = %path.display(),
                version = document.schema_version,
                "Leaving a settings document written by a newer build alone"
            );
            None
        }
        Stored::Unreadable => {
            tracing::warn!(
                path = %path.display(),
                "Leaving a settings document that cannot be read alone"
            );
            None
        }
        Stored::Missing => Some(Map::new()),
    }
}

async fn write_document(
    path: &Path,
    data: Map<String, Value>,
) -> crate::Result<()> {
    let document = Document {
        schema_version: SCHEMA_VERSION,
        written_by: env!("CARGO_PKG_VERSION").to_string(),
        data: Value::Object(data),
    };
    if let Some(parent) = path.parent() {
        crate::util::io::create_dir_all(parent).await?;
    }
    crate::util::io::write(path, serde_json::to_vec_pretty(&document)?).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh_settings() -> Settings {
        Settings::get().await
    }

    fn appearance() -> &'static [&'static str] {
        DOMAINS[0].1
    }

    fn document(data: &str) -> String {
        format!(
            r#"{{"schema_version":{SCHEMA_VERSION},"written_by":"9.9.9","data":{data}}}"#
        )
    }

    #[tokio::test]
    async fn every_stored_key_is_a_settings_field() {
        let value = serde_json::to_value(fresh_settings().await).unwrap();
        let fields = value.as_object().unwrap();

        for (name, keys) in DOMAINS {
            for key in *keys {
                assert!(fields.contains_key(*key), "{name} lists {key}");
            }
        }
    }

    #[tokio::test]
    async fn every_settings_field_belongs_to_a_domain() {
        let value = serde_json::to_value(fresh_settings().await).unwrap();
        let fields = value.as_object().unwrap();

        for field in fields.keys() {
            assert!(
                DOMAINS
                    .iter()
                    .any(|(_, keys)| keys.contains(&field.as_str())),
                "{field} belongs to no domain"
            );
        }
    }

    #[tokio::test]
    async fn stored_keys_shadow_the_defaults_after_a_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");

        let mut settings = fresh_settings().await;
        settings.accent_color = crate::state::AccentColor::Blue;
        store_to(&path, appearance(), &settings).await.unwrap();

        let merged =
            overlay_from(&path, appearance(), fresh_settings().await).await;
        assert_eq!(merged.accent_color, crate::state::AccentColor::Blue);
    }

    #[tokio::test]
    async fn a_fresh_installation_has_nothing_to_hand_over() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!needs_seeding_at(Some(dir.path()), false).await);
    }

    #[tokio::test]
    async fn an_installation_without_documents_takes_the_row_over() {
        let dir = tempfile::tempdir().unwrap();
        assert!(needs_seeding_at(Some(dir.path()), true).await);

        // An empty directory is a handover that stopped before its first
        // document, so the start that finds it writes them all.
        std::fs::create_dir_all(dir.path().join(DIR_NAME)).unwrap();
        assert!(needs_seeding_at(Some(dir.path()), true).await);
    }

    /// A handover is only on record once every document is in place, so the
    /// start that failed part way writes the rest.
    #[tokio::test]
    async fn a_handover_writes_every_document_before_it_is_on_record() {
        let dir = tempfile::tempdir().unwrap();
        TEST_DIR.with(|current| {
            *current.borrow_mut() = Some(dir.path().to_path_buf());
        });

        let settings = Settings::get().await;
        let proxy = Settings::proxy_entries(&ProxyConfig::default());
        assert!(hand_over(&settings, &proxy).await);
        for (name, _) in DOMAINS.iter().copied().chain([*PROXY_DOCUMENT]) {
            assert!(document_path(dir.path(), name).exists(), "{name}");
        }
        assert!(handover_marker(dir.path()).exists());
        assert!(!needs_seeding_at(Some(dir.path()), true).await);

        // The marker outlives a document the user deletes, which keeps the row
        // from filling it back in.
        std::fs::remove_file(document_path(dir.path(), "appearance")).unwrap();
        assert!(!needs_seeding_at(Some(dir.path()), true).await);

        TEST_DIR.with(|current| *current.borrow_mut() = None);
    }

    /// An installation whose documents are all in place took the row over
    /// before the marker existed.
    #[tokio::test]
    async fn a_complete_set_of_documents_needs_no_handover() {
        let dir = tempfile::tempdir().unwrap();
        for (name, _) in DOMAINS.iter().copied().chain([*PROXY_DOCUMENT]) {
            let path = document_path(dir.path(), name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, document("{}")).unwrap();
        }

        assert!(!handover_marker(dir.path()).exists());
        assert!(!needs_seeding_at(Some(dir.path()), true).await);
    }

    #[tokio::test]
    async fn a_store_without_a_directory_has_nothing_to_write_to() {
        assert!(!needs_seeding_at(None, true).await);
    }

    #[tokio::test]
    async fn a_key_that_matches_the_default_leaves_the_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);

        let mut settings = Settings::default();
        settings.theme = crate::state::Theme::Dark;
        settings.locale = "de-DE".to_string();
        store_to(&path, appearance(), &settings).await.unwrap();

        let stored = stored_at(&path).await;
        assert!(!stored.contains_key("theme"));
        assert_eq!(stored.get("locale"), Some(&Value::from("de-DE")));
    }

    /// A value the user also wrote into the deployment's document stops
    /// shadowing it, which is what lets a later change to it arrive.
    #[tokio::test]
    async fn a_local_entry_that_repeats_a_shipped_default_leaves_the_document()
    {
        let dir = tempfile::tempdir().unwrap();
        let shipped = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(DIR_NAME)).unwrap();
        let path = dir.path().join(DIR_NAME).join("appearance.json");
        std::fs::write(&path, document(r#"{"theme":"oled","locale":"fr-FR"}"#))
            .unwrap();
        std::fs::write(
            &shipped.path().join("appearance.json"),
            document(r#"{"theme":"oled"}"#),
        )
        .unwrap();

        TEST_DIR.with(|current| {
            *current.borrow_mut() = Some(dir.path().to_path_buf());
        });
        TEST_CONFIG_DIR.with(|current| {
            *current.borrow_mut() = Some(shipped.path().to_path_buf());
        });
        let settings = read_settings().await;

        assert_eq!(settings.theme.as_str(), "oled");
        assert_eq!(settings.locale, "fr-FR");
        let stored = stored_at(&path).await;
        assert!(!stored.contains_key("theme"));
        assert_eq!(stored.get("locale"), Some(&Value::from("fr-FR")));

        TEST_DIR.with(|current| *current.borrow_mut() = None);
        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);
    }

    /// An entry a document carries in a shape this build cannot read costs only
    /// itself, which keeps the rest of the document in the settings.
    #[tokio::test]
    async fn an_entry_that_cannot_be_read_costs_only_itself() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(DIR_NAME)).unwrap();
        std::fs::write(
            dir.path().join(DIR_NAME).join("appearance.json"),
            document(r#"{"theme":"oled","locale":5,"log_level":"trace"}"#),
        )
        .unwrap();
        TEST_DIR.with(|current| {
            *current.borrow_mut() = Some(dir.path().to_path_buf());
        });
        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);

        let settings = read_settings().await;
        assert_eq!(settings.theme.as_str(), "oled");
        assert_eq!(settings.log_level, "trace");
        assert_eq!(settings.locale, Settings::default().locale);

        TEST_DIR.with(|current| *current.borrow_mut() = None);
    }

    /// A document that leaves a key out reads as the default the build carries.
    #[tokio::test]
    async fn a_key_a_document_leaves_out_reads_as_the_default() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(DIR_NAME)).unwrap();
        std::fs::write(
            dir.path().join(DIR_NAME).join("appearance.json"),
            document(r#"{"locale":"de-DE"}"#),
        )
        .unwrap();
        TEST_DIR.with(|current| {
            *current.borrow_mut() = Some(dir.path().to_path_buf());
        });
        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);

        let settings = read_settings().await;
        assert_eq!(settings.locale, "de-DE");
        assert_eq!(settings.log_level, crate::logger::DEFAULT_LOG_LEVEL);

        TEST_DIR.with(|current| *current.borrow_mut() = None);
    }

    #[tokio::test]
    async fn a_key_the_deployment_ships_stays_out_of_the_document() {
        let dir = tempfile::tempdir().unwrap();
        let shipped = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(
            &shipped.path().join("appearance.json"),
            document(r#"{"theme":"oled"}"#),
        )
        .unwrap();
        TEST_CONFIG_DIR.with(|current| {
            *current.borrow_mut() = Some(shipped.path().to_path_buf());
        });

        let mut settings = fresh_settings().await;
        settings.theme = crate::state::Theme::Oled;
        settings.locale = "de-DE".to_string();
        store_to(&path, appearance(), &settings).await.unwrap();

        let stored = stored_at(&path).await;
        assert!(!stored.contains_key("theme"));
        assert_eq!(stored.get("locale"), Some(&Value::from("de-DE")));

        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);
    }

    #[tokio::test]
    async fn a_key_that_returns_to_a_shipped_default_leaves_the_document() {
        let dir = tempfile::tempdir().unwrap();
        let shipped = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(&path, document(r#"{"theme":"oled","locale":"de-DE"}"#))
            .unwrap();
        std::fs::write(
            &shipped.path().join("appearance.json"),
            document(r#"{"theme":"dark"}"#),
        )
        .unwrap();
        TEST_CONFIG_DIR.with(|current| {
            *current.borrow_mut() = Some(shipped.path().to_path_buf());
        });

        let mut settings = fresh_settings().await;
        settings.theme = crate::state::Theme::Dark;
        settings.locale = "de-DE".to_string();
        store_to(&path, appearance(), &settings).await.unwrap();

        let stored = stored_at(&path).await;
        assert!(!stored.contains_key("theme"));
        assert_eq!(stored.get("locale"), Some(&Value::from("de-DE")));

        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);
    }

    #[tokio::test]
    async fn a_single_key_write_leaves_a_shipped_default_alone() {
        let dir = tempfile::tempdir().unwrap();
        let shipped = tempfile::tempdir().unwrap();
        let path = dir.path().join("privacy.json");
        std::fs::write(
            &shipped.path().join("privacy.json"),
            document(r#"{"telemetry":true}"#),
        )
        .unwrap();
        TEST_CONFIG_DIR.with(|current| {
            *current.borrow_mut() = Some(shipped.path().to_path_buf());
        });

        set_key_at(&path, "telemetry", Value::Bool(true))
            .await
            .unwrap();
        assert!(!stored_at(&path).await.contains_key("telemetry"));

        set_key_at(&path, "telemetry", Value::Bool(false))
            .await
            .unwrap();
        assert_eq!(
            stored_at(&path).await.get("telemetry"),
            Some(&Value::Bool(false))
        );

        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);
    }

    #[tokio::test]
    async fn a_default_document_gives_way_to_the_local_one() {
        let dir = tempfile::tempdir().unwrap();
        let defaults = dir.path().join("defaults.json");
        let local = dir.path().join("local.json");
        std::fs::write(
            &defaults,
            document(r#"{"locale":"de-DE","theme":"oled"}"#),
        )
        .unwrap();
        std::fs::write(&local, document(r#"{"locale":"fr-FR"}"#)).unwrap();

        let mut stored = fresh_settings().await;
        stored.locale = "en-US".to_string();
        stored.theme = crate::state::Theme::Dark;
        let merged =
            overlay_layers(Some(&defaults), Some(&local), appearance(), stored)
                .await;

        assert_eq!(merged.locale, "fr-FR");
        assert_eq!(merged.theme.as_str(), "oled");
    }

    #[tokio::test]
    async fn a_default_document_applies_without_a_local_one() {
        let dir = tempfile::tempdir().unwrap();
        let defaults = dir.path().join("defaults.json");
        std::fs::write(&defaults, document(r#"{"locale":"de-DE"}"#)).unwrap();

        let mut stored = fresh_settings().await;
        stored.locale = "en-US".to_string();
        let merged =
            overlay_layers(Some(&defaults), None, appearance(), stored).await;
        assert_eq!(merged.locale, "de-DE");
    }

    #[tokio::test]
    async fn stored_entries_layer_a_default_under_the_local_file() {
        let dir = tempfile::tempdir().unwrap();
        let defaults = dir.path().join("defaults.json");
        let local = dir.path().join("local.json");
        std::fs::write(
            &defaults,
            document(r#"{"proxy_mode":"system","proxy_url":"http://default"}"#),
        )
        .unwrap();
        std::fs::write(&local, document(r#"{"proxy_mode":"custom"}"#)).unwrap();

        let entries = stored_layers(Some(&defaults), Some(&local)).await;
        assert_eq!(entries.get("proxy_mode"), Some(&Value::from("custom")));
        assert_eq!(
            entries.get("proxy_url"),
            Some(&Value::from("http://default"))
        );
    }

    #[tokio::test]
    async fn keys_no_document_carries_stay_with_the_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");

        let mut settings = fresh_settings().await;
        settings.custom_dir = Some("/tmp/kept".to_string());
        store_to(&path, appearance(), &settings).await.unwrap();
        assert!(
            !std::fs::read_to_string(&path)
                .unwrap()
                .contains("custom_dir")
        );

        let mut stored = fresh_settings().await;
        stored.custom_dir = Some("/tmp/row".to_string());
        let merged = overlay_from(&path, appearance(), stored).await;
        assert_eq!(merged.custom_dir.as_deref(), Some("/tmp/row"));
    }

    #[tokio::test]
    async fn a_write_keeps_the_keys_this_build_does_not_know() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(
            &path,
            document(r#"{"theme":"oled","from_a_newer_build":7}"#),
        )
        .unwrap();

        let mut settings = fresh_settings().await;
        settings.accent_color = crate::state::AccentColor::Blue;
        store_to(&path, appearance(), &settings).await.unwrap();

        let stored: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
                .unwrap();
        assert_eq!(stored["data"]["from_a_newer_build"], 7);
        assert_eq!(stored["data"]["accent_color"], "blue");
        assert_eq!(stored["written_by"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn a_document_from_a_newer_build_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        let newer = format!(
            r#"{{"schema_version":{},"written_by":"9.9.9","data":{{"theme":"oled"}}}}"#,
            SCHEMA_VERSION + 1
        );
        std::fs::write(&path, &newer).unwrap();

        store_to(&path, appearance(), &fresh_settings().await)
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), newer);
    }

    #[tokio::test]
    async fn sanitise_drops_the_keys_no_domain_knows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(
            &path,
            document(r#"{"theme":"oled","from_a_newer_build":7}"#),
        )
        .unwrap();

        assert_eq!(sanitise_at(&path, appearance()).await.unwrap(), 1);

        let stored: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
                .unwrap();
        assert_eq!(stored["data"]["theme"], "oled");
        assert!(stored["data"].get("from_a_newer_build").is_none());
    }

    #[tokio::test]
    async fn sanitise_leaves_a_newer_document_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        let newer = format!(
            r#"{{"schema_version":{},"written_by":"9.9.9","data":{{"from_a_newer_build":7}}}}"#,
            SCHEMA_VERSION + 1
        );
        std::fs::write(&path, &newer).unwrap();

        assert_eq!(sanitise_at(&path, appearance()).await.unwrap(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), newer);
    }

    #[tokio::test]
    async fn a_single_key_write_keeps_the_rest_of_the_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.json");
        std::fs::write(
            &path,
            document(r#"{"theme":"oled","from_a_newer_build":7}"#),
        )
        .unwrap();

        set_key_at(&path, "force_fullscreen", Value::Bool(true))
            .await
            .unwrap();

        let stored: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
                .unwrap();
        assert_eq!(stored["data"]["force_fullscreen"], true);
        assert_eq!(stored["data"]["theme"], "oled");
        assert_eq!(stored["data"]["from_a_newer_build"], 7);
    }

    #[tokio::test]
    async fn entries_a_domain_owns_are_stored_alongside_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("proxy.json");
        std::fs::write(&path, document(r#"{"proxy_mode":"system"}"#)).unwrap();

        set_entries_at(
            &path,
            &[
                ("proxy_mode", Value::from("custom")),
                ("proxy_url", Value::from("http://localhost:8080")),
            ],
        )
        .await
        .unwrap();

        let stored = stored_at(&path).await;
        assert_eq!(stored.get("proxy_mode"), Some(&Value::from("custom")));
        assert_eq!(
            stored.get("proxy_url"),
            Some(&Value::from("http://localhost:8080"))
        );
    }

    /// A domain that owns its keys follows the defaults just like the others.
    #[tokio::test]
    async fn entries_a_domain_owns_follow_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("proxy.json");
        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);

        set_entries_at(
            &path,
            &[
                ("proxy_mode", Value::from("custom")),
                ("proxy_url", Value::from("")),
            ],
        )
        .await
        .unwrap();

        let stored = stored_at(&path).await;
        assert_eq!(stored.get("proxy_mode"), Some(&Value::from("custom")));
        assert!(!stored.contains_key("proxy_url"));
    }

    /// Startup drops what a document repeats of a default, which is what makes
    /// a default that changed while the launcher was closed arrive.
    #[tokio::test]
    async fn startup_drops_the_entries_a_default_provides() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(DIR_NAME)).unwrap();
        let path = dir.path().join(DIR_NAME).join("appearance.json");
        std::fs::write(&path, document(r#"{"theme":"dark","locale":"fr-FR"}"#))
            .unwrap();
        TEST_DIR.with(|current| {
            *current.borrow_mut() = Some(dir.path().to_path_buf());
        });
        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);

        prune_redundant().await;

        let stored = stored_at(&path).await;
        assert!(!stored.contains_key("theme"));
        assert_eq!(stored.get("locale"), Some(&Value::from("fr-FR")));

        TEST_DIR.with(|current| *current.borrow_mut() = None);
    }

    /// The proxy document is pruned next to the domains, which is what keeps a
    /// local entry from shadowing a deployment's value forever.
    #[tokio::test]
    async fn the_proxy_document_is_pruned_with_the_domains() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(DIR_NAME)).unwrap();
        let path = dir.path().join(DIR_NAME).join("proxy.json");
        std::fs::write(
            &path,
            document(r#"{"proxy_mode":"system","proxy_url":"http://p:1"}"#),
        )
        .unwrap();
        TEST_DIR.with(|current| {
            *current.borrow_mut() = Some(dir.path().to_path_buf());
        });
        TEST_CONFIG_DIR.with(|current| *current.borrow_mut() = None);

        prune_redundant().await;

        let stored = stored_at(&path).await;
        assert!(!stored.contains_key("proxy_mode"));
        assert_eq!(stored.get("proxy_url"), Some(&Value::from("http://p:1")));

        TEST_DIR.with(|current| *current.borrow_mut() = None);
    }

    #[tokio::test]
    async fn stored_reads_the_entries_of_a_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("privacy.json");
        std::fs::write(&path, document(r#"{"telemetry":true,"other":1}"#))
            .unwrap();

        let stored = stored_at(&path).await;
        assert_eq!(stored.get("telemetry"), Some(&Value::Bool(true)));
        assert_eq!(stored.get("other"), Some(&Value::from(1)));
    }

    #[tokio::test]
    async fn stored_is_empty_when_there_is_nothing_to_read() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("privacy.json");
        assert!(stored_at(&missing).await.is_empty());

        let corrupt = dir.path().join("corrupt.json");
        std::fs::write(&corrupt, b"{ not json").unwrap();
        assert!(stored_at(&corrupt).await.is_empty());
    }

    #[tokio::test]
    async fn a_reference_to_something_removed_is_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(
            &path,
            document(r#"{"minimal_home_instance_id":"gone","other":1}"#),
        )
        .unwrap();

        clear_key_if_at(&path, "minimal_home_instance_id", "gone")
            .await
            .unwrap();

        let stored = stored_at(&path).await;
        assert!(!stored.contains_key("minimal_home_instance_id"));
        assert_eq!(stored.get("other"), Some(&Value::from(1)));
    }

    #[tokio::test]
    async fn a_reference_to_something_else_stays() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(
            &path,
            document(r#"{"minimal_home_instance_id":"kept"}"#),
        )
        .unwrap();

        clear_key_if_at(&path, "minimal_home_instance_id", "gone")
            .await
            .unwrap();
        assert_eq!(
            stored_at(&path).await.get("minimal_home_instance_id"),
            Some(&Value::from("kept"))
        );
    }

    #[tokio::test]
    async fn a_missing_document_leaves_the_row_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");

        let mut stored = fresh_settings().await;
        stored.locale = "de-DE".to_string();
        let merged = overlay_from(&path, appearance(), stored).await;
        assert_eq!(merged.locale, "de-DE");
    }

    #[tokio::test]
    async fn a_document_from_a_newer_build_is_read_but_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        let newer = format!(
            r#"{{"schema_version":{},"written_by":"9.9.9","data":{{"locale":"de-DE"}}}}"#,
            SCHEMA_VERSION + 1
        );
        std::fs::write(&path, &newer).unwrap();

        let mut stored = fresh_settings().await;
        stored.locale = "fr-FR".to_string();
        let merged = overlay_from(&path, appearance(), stored).await;
        assert_eq!(merged.locale, "de-DE");
        assert_eq!(
            stored_at(&path).await.get("locale"),
            Some(&Value::from("de-DE"))
        );

        store_to(&path, appearance(), &fresh_settings().await)
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), newer);
    }

    #[tokio::test]
    async fn a_corrupt_document_is_left_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(&path, b"{ not json").unwrap();

        let mut stored = fresh_settings().await;
        stored.locale = "fr-FR".to_string();
        let merged = overlay_from(&path, appearance(), stored).await;
        assert_eq!(merged.locale, "fr-FR");
        assert!(path.exists());
    }

    #[tokio::test]
    async fn a_corrupt_document_is_neither_written_nor_cleaned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        std::fs::write(&path, b"{ not json").unwrap();

        store_to(&path, appearance(), &fresh_settings().await)
            .await
            .unwrap();
        assert_eq!(sanitise_at(&path, appearance()).await.unwrap(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
    }
}
