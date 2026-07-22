//! Theseus settings file

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

// Types
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, Eq, PartialEq,
)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum DownloadSourceMode {
    #[default]
    Auto = 0,
    OfficialOnly = 1,
    MirrorPreferred = 2,
}

impl DownloadSourceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::OfficialOnly => "official_only",
            Self::MirrorPreferred => "mirror_preferred",
        }
    }

    pub fn from_string(value: &str) -> Self {
        match value {
            "official_only" => Self::OfficialOnly,
            "mirror_preferred" => Self::MirrorPreferred,
            _ => Self::Auto,
        }
    }

    pub(crate) fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::OfficialOnly,
            2 => Self::MirrorPreferred,
            _ => Self::Auto,
        }
    }

    pub(crate) fn prefers_mirror(self, auto_prefers_mirror: bool) -> bool {
        match self {
            Self::Auto => auto_prefers_mirror,
            Self::OfficialOnly => false,
            Self::MirrorPreferred => true,
        }
    }
}

/// Global Theseus settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub max_concurrent_downloads: usize,
    pub max_concurrent_writes: usize,
    #[serde(default)]
    pub auto_concurrent_downloads: bool,
    #[serde(default)]
    pub minecraft_metadata_source: DownloadSourceMode,
    #[serde(default)]
    pub minecraft_file_source: DownloadSourceMode,
    #[serde(default)]
    pub modrinth_source: DownloadSourceMode,
    #[serde(default)]
    pub curseforge_source: DownloadSourceMode,
    #[serde(default, rename = "use_minecraft_mirror", skip_serializing)]
    legacy_use_minecraft_mirror: Option<bool>,
    #[serde(default, rename = "use_modrinth_mirror", skip_serializing)]
    legacy_use_modrinth_mirror: Option<bool>,
    #[serde(default, rename = "use_curseforge_mirror", skip_serializing)]
    legacy_use_curseforge_mirror: Option<bool>,

    pub theme: Theme,
    pub accent_color: AccentColor,
    pub locale: String,
    pub default_page: DefaultPage,
    pub collapsed_navigation: bool,
    pub hide_nametag_skins_page: bool,
    pub advanced_rendering: bool,
    pub native_decorations: bool,
    pub toggle_sidebar: bool,
    pub custom_background_path: Option<String>,
    pub custom_background_blur: u32,
    pub custom_background_opacity: u32,

    pub telemetry: bool,
    pub discord_rpc: bool,
    #[serde(skip, default)]
    pub personalized_ads: bool,

    pub onboarded: bool,

    pub extra_launch_args: Vec<String>,
    pub custom_env_vars: Vec<(String, String)>,
    pub memory: MemorySettings,
    pub force_fullscreen: bool,
    pub game_resolution: WindowSize,
    pub hide_on_process_start: bool,
    pub hooks: Hooks,

    pub custom_dir: Option<String>,
    pub prev_custom_dir: Option<String>,
    pub migrated: bool,

    pub developer_mode: bool,
    pub feature_flags: HashMap<FeatureFlag, bool>,

    pub skipped_update: Option<String>,
    pub pending_update_toast_for_version: Option<String>,
    pub auto_download_updates: Option<bool>,

    pub version: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    PagePath,
    ProjectBackground,
    WorldsTab,
    WorldsInHome,
    ServerRamAsBytesAlwaysOn,
    AlwaysShowAppControls,
    SkipUnknownPackWarning,
    PrideFundraiser,
    ServersInApp,
    ServerProjectQa,
    I18nDebug,
    ShowInstancePlayTime,
    SkipNonEssentialWarnings,
    AdvancedFiltersCollapsed,
}

impl Settings {
    const CURRENT_VERSION: usize = 3;

    pub async fn get(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Self> {
        let res = sqlx::query!(
            "
            SELECT
                max_concurrent_writes, max_concurrent_downloads,
                auto_concurrent_downloads, minecraft_metadata_source,
                minecraft_file_source, modrinth_source, curseforge_source,
                theme, locale, default_page, collapsed_navigation, hide_nametag_skins_page, advanced_rendering, native_decorations,
                discord_rpc, developer_mode, telemetry, personalized_ads,
                onboarded,
                json(extra_launch_args) extra_launch_args, json(custom_env_vars) custom_env_vars,
                mc_memory_max, mc_force_fullscreen, mc_game_resolution_x, mc_game_resolution_y, hide_on_process_start,
                hook_pre_launch, hook_wrapper, hook_post_exit,
                custom_dir, prev_custom_dir, migrated, json(feature_flags) feature_flags, toggle_sidebar,
                skipped_update, pending_update_toast_for_version, auto_download_updates, accent_color,
                custom_background_path, custom_background_blur, custom_background_opacity,
                version
            FROM settings
            "
        )
            .fetch_one(exec)
            .await?;

        Ok(Self {
            max_concurrent_downloads: res.max_concurrent_downloads as usize,
            max_concurrent_writes: res.max_concurrent_writes as usize,
            auto_concurrent_downloads: res.auto_concurrent_downloads == 1,
            minecraft_metadata_source: DownloadSourceMode::from_string(
                &res.minecraft_metadata_source,
            ),
            minecraft_file_source: DownloadSourceMode::from_string(
                &res.minecraft_file_source,
            ),
            modrinth_source: DownloadSourceMode::from_string(
                &res.modrinth_source,
            ),
            curseforge_source: DownloadSourceMode::from_string(
                &res.curseforge_source,
            ),
            legacy_use_minecraft_mirror: None,
            legacy_use_modrinth_mirror: None,
            legacy_use_curseforge_mirror: None,
            theme: Theme::from_string(&res.theme),
            accent_color: AccentColor::from_string(&res.accent_color),
            locale: res.locale,
            default_page: DefaultPage::from_string(&res.default_page),
            collapsed_navigation: res.collapsed_navigation == 1,
            hide_nametag_skins_page: res.hide_nametag_skins_page == 1,
            advanced_rendering: res.advanced_rendering == 1,
            native_decorations: res.native_decorations == 1,
            toggle_sidebar: res.toggle_sidebar == 1,
            custom_background_path: res.custom_background_path,
            custom_background_blur: res.custom_background_blur as u32,
            custom_background_opacity: res.custom_background_opacity as u32,
            telemetry: res.telemetry == 1,
            discord_rpc: res.discord_rpc == 1,
            developer_mode: res.developer_mode == 1,
            personalized_ads: res.personalized_ads == 1,
            onboarded: res.onboarded == 1,
            extra_launch_args: res
                .extra_launch_args
                .as_ref()
                .and_then(|x| serde_json::from_str(x).ok())
                .unwrap_or_default(),
            custom_env_vars: res
                .custom_env_vars
                .as_ref()
                .and_then(|x| serde_json::from_str(x).ok())
                .unwrap_or_default(),
            memory: MemorySettings {
                maximum: res.mc_memory_max as u32,
            },
            force_fullscreen: res.mc_force_fullscreen == 1,
            game_resolution: WindowSize(
                res.mc_game_resolution_x as u16,
                res.mc_game_resolution_y as u16,
            ),
            hide_on_process_start: res.hide_on_process_start == 1,
            hooks: Hooks {
                pre_launch: res.hook_pre_launch,
                wrapper: res.hook_wrapper,
                post_exit: res.hook_post_exit,
            },
            custom_dir: res.custom_dir,
            prev_custom_dir: res.prev_custom_dir,
            migrated: res.migrated == 1,
            feature_flags: res
                .feature_flags
                .as_ref()
                .and_then(|x| serde_json::from_str(x).ok())
                .unwrap_or_default(),
            skipped_update: res.skipped_update,
            pending_update_toast_for_version: res
                .pending_update_toast_for_version,
            auto_download_updates: res.auto_download_updates.map(|x| x == 1),
            version: res.version as usize,
        })
    }

    pub async fn update(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let max_concurrent_writes = self.max_concurrent_writes as i32;
        let max_concurrent_downloads =
            self.max_concurrent_downloads.clamp(1, 256) as i32;
        let theme = self.theme.as_str();
        let accent_color = self.accent_color.as_str();
        let default_page = self.default_page.as_str();
        let extra_launch_args = serde_json::to_string(&self.extra_launch_args)?;
        let custom_env_vars = serde_json::to_string(&self.custom_env_vars)?;
        let feature_flags = serde_json::to_string(&self.feature_flags)?;
        let custom_background_blur = self.custom_background_blur.min(40) as i32;
        let custom_background_opacity =
            self.custom_background_opacity.clamp(10, 100) as i32;
        let version = self.version as i64;
        let minecraft_metadata_source = self.minecraft_metadata_source.as_str();
        let minecraft_file_source = self.minecraft_file_source.as_str();
        let modrinth_source = self.modrinth_source.as_str();
        let curseforge_source = self.curseforge_source.as_str();
        let auto_prefers_mirror = self.auto_prefers_mirror();
        let use_minecraft_mirror = self
            .minecraft_file_source
            .prefers_mirror(auto_prefers_mirror);
        let use_modrinth_mirror =
            self.modrinth_source.prefers_mirror(auto_prefers_mirror);
        let use_curseforge_mirror =
            self.curseforge_source.prefers_mirror(auto_prefers_mirror);

        sqlx::query!(
            "
            UPDATE settings
            SET
                max_concurrent_writes = $1,
                max_concurrent_downloads = $2,

                theme = $3,
                locale = $4,
                default_page = $5,
                collapsed_navigation = $6,
                advanced_rendering = $7,
                native_decorations = $8,

                discord_rpc = $9,
                developer_mode = $10,
                telemetry = $11,
                personalized_ads = $12,

                onboarded = $13,

                extra_launch_args = jsonb($14),
                custom_env_vars = jsonb($15),
                mc_memory_max = $16,
                mc_force_fullscreen = $17,
                mc_game_resolution_x = $18,
                mc_game_resolution_y = $19,
                hide_on_process_start = $20,

                hook_pre_launch = $21,
                hook_wrapper = $22,
                hook_post_exit = $23,

                custom_dir = $24,
                prev_custom_dir = $25,
                migrated = $26,

                toggle_sidebar = $27,
                feature_flags = $28,
                hide_nametag_skins_page = $29,

                skipped_update = $30,
                pending_update_toast_for_version = $31,
                auto_download_updates = $32,
                accent_color = $33,
                custom_background_path = $34,
                custom_background_blur = $35,
                custom_background_opacity = $36,

                version = $37,
                auto_concurrent_downloads = $38,
                minecraft_metadata_source = $39,
                minecraft_file_source = $40,
                modrinth_source = $41,
                curseforge_source = $42,
                use_minecraft_mirror = $43,
                use_modrinth_mirror = $44,
                use_curseforge_mirror = $45
            ",
            max_concurrent_writes,
            max_concurrent_downloads,
            theme,
            self.locale,
            default_page,
            self.collapsed_navigation,
            self.advanced_rendering,
            self.native_decorations,
            self.discord_rpc,
            self.developer_mode,
            self.telemetry,
            self.personalized_ads,
            self.onboarded,
            extra_launch_args,
            custom_env_vars,
            self.memory.maximum,
            self.force_fullscreen,
            self.game_resolution.0,
            self.game_resolution.1,
            self.hide_on_process_start,
            self.hooks.pre_launch,
            self.hooks.wrapper,
            self.hooks.post_exit,
            self.custom_dir,
            self.prev_custom_dir,
            self.migrated,
            self.toggle_sidebar,
            feature_flags,
            self.hide_nametag_skins_page,
            self.skipped_update,
            self.pending_update_toast_for_version,
            self.auto_download_updates,
            accent_color,
            self.custom_background_path,
            custom_background_blur,
            custom_background_opacity,
            version,
            self.auto_concurrent_downloads,
            minecraft_metadata_source,
            minecraft_file_source,
            modrinth_source,
            curseforge_source,
            use_minecraft_mirror,
            use_modrinth_mirror,
            use_curseforge_mirror,
        )
        .execute(exec)
        .await?;

        Ok(())
    }

    pub fn effective_max_concurrent_downloads(&self) -> usize {
        if self.auto_concurrent_downloads {
            64
        } else {
            self.max_concurrent_downloads.clamp(1, 256)
        }
    }

    pub(crate) fn apply_legacy_download_source_settings(&mut self) {
        let has_legacy_settings = self.legacy_use_minecraft_mirror.is_some()
            || self.legacy_use_modrinth_mirror.is_some()
            || self.legacy_use_curseforge_mirror.is_some();
        let has_explicit_source_settings = self.minecraft_metadata_source
            != DownloadSourceMode::Auto
            || self.minecraft_file_source != DownloadSourceMode::Auto
            || self.modrinth_source != DownloadSourceMode::Auto
            || self.curseforge_source != DownloadSourceMode::Auto;
        if !has_legacy_settings || has_explicit_source_settings {
            return;
        }

        if self.legacy_use_minecraft_mirror == Some(false)
            && self.legacy_use_modrinth_mirror == Some(false)
            && self.legacy_use_curseforge_mirror == Some(true)
        {
            self.minecraft_metadata_source = DownloadSourceMode::Auto;
            self.minecraft_file_source = DownloadSourceMode::Auto;
            self.modrinth_source = DownloadSourceMode::Auto;
            self.curseforge_source = DownloadSourceMode::Auto;
            return;
        }

        if let Some(enabled) = self.legacy_use_minecraft_mirror {
            let source = legacy_download_source(enabled);
            self.minecraft_metadata_source = source;
            self.minecraft_file_source = source;
        }
        if let Some(enabled) = self.legacy_use_modrinth_mirror {
            self.modrinth_source = legacy_download_source(enabled);
        }
        if let Some(enabled) = self.legacy_use_curseforge_mirror {
            self.curseforge_source = legacy_download_source(enabled);
        }
    }

    pub(crate) fn auto_prefers_mirror(&self) -> bool {
        let timezone = std::env::var("TZ").ok().or_else(|| {
            std::fs::read_link("/etc/localtime")
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        });

        if let Some(timezone) = timezone {
            return locale_prefers_mirror(&timezone);
        }

        locale_prefers_mirror(&self.locale)
            || ["LC_ALL", "LC_MESSAGES", "LANG"]
                .into_iter()
                .filter_map(|key| std::env::var(key).ok())
                .any(|value| locale_prefers_mirror(&value))
    }

    pub async fn migrate(exec: &Pool<Sqlite>) -> crate::Result<()> {
        let mut settings = Self::get(exec).await?;

        if settings.version < Settings::CURRENT_VERSION {
            tracing::info!(
                "Migrating settings version {} to {:?}",
                settings.version,
                Settings::CURRENT_VERSION
            );
        }
        while settings.version < Settings::CURRENT_VERSION {
            if let Err(err) = settings.perform_migration() {
                tracing::error!(
                    "Failed to migrate settings from version {}: {}",
                    settings.version,
                    err
                );
                return Err(err);
            }
        }

        settings.update(exec).await?;

        Ok(())
    }

    pub fn perform_migration(&mut self) -> crate::Result<()> {
        match self.version {
            1 => {
                let quoter = shlex::Quoter::new().allow_nul(true);

                // Previously split by spaces
                if let Some(pre_launch) = self.hooks.pre_launch.as_ref() {
                    self.hooks.pre_launch =
                        Some(quoter.join(pre_launch.split(' ')).unwrap())
                }

                // Previously treated as complete path to command
                if let Some(wrapper) = self.hooks.wrapper.as_ref() {
                    self.hooks.wrapper =
                        Some(quoter.quote(wrapper).unwrap().to_string())
                }

                // Previously split by spaces
                if let Some(post_exit) = self.hooks.post_exit.as_ref() {
                    self.hooks.post_exit =
                        Some(quoter.join(post_exit.split(' ')).unwrap())
                }

                self.version = 2;
            }
            2 => {
                // Update old default memory setting from 2GB to 4GB (depending on system memory)
                const LEGACY_DEFAULT_MEMORY_MB: u32 = 2048;
                if self.memory.maximum == LEGACY_DEFAULT_MEMORY_MB {
                    self.memory.maximum =
                        crate::api::jre::default_memory_max_mb();
                }

                self.version = 3;
            }
            version => {
                return Err(crate::ErrorKind::OtherError(format!(
                    "Invalid settings version: {version}"
                ))
                .into());
            }
        }

        Ok(())
    }
}

fn locale_prefers_mirror(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace('_', "-");

    normalized.starts_with("zh-cn")
        || normalized.starts_with("zh-hans")
        || normalized.contains("asia/shanghai")
        || normalized.contains("asia/chongqing")
        || normalized.contains("asia/harbin")
        || normalized.contains("asia/urumqi")
}

fn legacy_download_source(enabled: bool) -> DownloadSourceMode {
    if enabled {
        DownloadSourceMode::MirrorPreferred
    } else {
        DownloadSourceMode::OfficialOnly
    }
}

/// Accent color used for interactive controls and highlights.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentColor {
    Pink,
    Orange,
    Green,
    Blue,
    Purple,
}

impl AccentColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccentColor::Pink => "pink",
            AccentColor::Orange => "orange",
            AccentColor::Green => "green",
            AccentColor::Blue => "blue",
            AccentColor::Purple => "purple",
        }
    }

    pub fn from_string(string: &str) -> AccentColor {
        match string {
            "orange" => AccentColor::Orange,
            "green" => AccentColor::Green,
            "blue" => AccentColor::Blue,
            "purple" => AccentColor::Purple,
            _ => AccentColor::Pink,
        }
    }
}

/// Theseus theme
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    Oled,
    System,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Oled => "oled",
            Theme::System => "system",
        }
    }

    pub fn from_string(string: &str) -> Theme {
        match string {
            "dark" => Theme::Dark,
            "light" => Theme::Light,
            "oled" => Theme::Oled,
            "system" => Theme::System,
            _ => Theme::Dark,
        }
    }
}

/// Minecraft memory settings
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MemorySettings {
    pub maximum: u32,
}

/// Game window size
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct WindowSize(pub u16, pub u16);

/// Game initialization hooks
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde_with::serde_as]
pub struct Hooks {
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    pub pre_launch: Option<String>,
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    pub wrapper: Option<String>,
    #[serde_as(as = "serde_with::NoneAsEmptyString")]
    pub post_exit: Option<String>,
}

/// Opening window to start with
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DefaultPage {
    Home,
    DiscoverContent,
    Library,
}

impl DefaultPage {
    pub fn as_str(&self) -> &'static str {
        match self {
            DefaultPage::Home => "home",
            DefaultPage::DiscoverContent => "discover_content",
            DefaultPage::Library => "library",
        }
    }

    pub fn from_string(string: &str) -> Self {
        match string {
            "home" => Self::Home,
            "discover_content" => Self::DiscoverContent,
            "library" => Self::Library,
            _ => Self::Home,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_source_mode_uses_stable_wire_values() {
        assert_eq!(
            serde_json::to_string(&DownloadSourceMode::Auto).unwrap(),
            "\"auto\""
        );
        assert_eq!(
            serde_json::to_string(&DownloadSourceMode::OfficialOnly).unwrap(),
            "\"official_only\""
        );
        assert_eq!(
            serde_json::to_string(&DownloadSourceMode::MirrorPreferred)
                .unwrap(),
            "\"mirror_preferred\""
        );
    }

    #[test]
    fn auto_source_detection_distinguishes_mainland_locales() {
        assert!(locale_prefers_mirror("zh-CN"));
        assert!(locale_prefers_mirror("zh_Hans"));
        assert!(locale_prefers_mirror("Asia/Shanghai"));
        assert!(!locale_prefers_mirror("zh-TW"));
        assert!(!locale_prefers_mirror("en-US"));
    }

    #[tokio::test]
    async fn legacy_mirror_settings_keep_their_previous_intent() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let settings = Settings::get(&pool).await.unwrap();
        assert!(settings.auto_concurrent_downloads);
        assert_eq!(
            settings.minecraft_metadata_source,
            DownloadSourceMode::Auto
        );
        assert_eq!(settings.minecraft_file_source, DownloadSourceMode::Auto);
        assert_eq!(settings.modrinth_source, DownloadSourceMode::Auto);
        assert_eq!(settings.curseforge_source, DownloadSourceMode::Auto);
        let mut legacy = serde_json::to_value(settings).unwrap();
        let object = legacy.as_object_mut().unwrap();
        object.remove("minecraft_metadata_source");
        object.remove("minecraft_file_source");
        object.remove("modrinth_source");
        object.remove("curseforge_source");
        object.insert("use_minecraft_mirror".to_string(), true.into());
        object.insert("use_modrinth_mirror".to_string(), false.into());
        object.insert("use_curseforge_mirror".to_string(), false.into());

        let mut migrated: Settings = serde_json::from_value(legacy).unwrap();
        migrated.apply_legacy_download_source_settings();

        assert_eq!(
            migrated.minecraft_metadata_source,
            DownloadSourceMode::MirrorPreferred
        );
        assert_eq!(
            migrated.minecraft_file_source,
            DownloadSourceMode::MirrorPreferred
        );
        assert_eq!(migrated.modrinth_source, DownloadSourceMode::OfficialOnly);
        assert_eq!(
            migrated.curseforge_source,
            DownloadSourceMode::OfficialOnly
        );
    }

    #[tokio::test]
    async fn download_source_reset_migration_sets_all_sources_to_auto() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        sqlx::query(
            "
            UPDATE settings
            SET
                minecraft_metadata_source = 'official_only',
                minecraft_file_source = 'mirror_preferred',
                modrinth_source = 'official_only',
                curseforge_source = 'mirror_preferred'
            ",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/migrations/20260721120000_reset-download-sources-to-auto.sql"
        )))
        .execute(&pool)
        .await
        .unwrap();

        let settings = Settings::get(&pool).await.unwrap();
        assert_eq!(
            settings.minecraft_metadata_source,
            DownloadSourceMode::Auto
        );
        assert_eq!(settings.minecraft_file_source, DownloadSourceMode::Auto);
        assert_eq!(settings.modrinth_source, DownloadSourceMode::Auto);
        assert_eq!(settings.curseforge_source, DownloadSourceMode::Auto);
    }
}
