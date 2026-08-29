//! Logic for launching Minecraft
use crate::data::ModLoader;
use crate::event::emit::{emit_instance, emit_loading, init_loading};
use crate::event::{InstancePayloadType, LoadingBarType};
use crate::install::{
    InstallJavaStep, InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter,
};
use crate::instance::QuickPlayType;
use crate::launcher::download::{LocalRuntimeSource, download_log_config};
use crate::launcher::quick_play_version::{
    QuickPlayServerVersion, QuickPlayVersion,
};
use crate::server_address::{ServerAddress, parse_server_address};
use crate::state::server_join_log::JoinLogEntry;
use crate::state::{
    CacheBehaviour, Credentials, InstanceInstallStage, InstanceLaunchContext,
    InstanceLink, JavaVersion, MemorySettings, ProcessMetadata, WindowSize,
};
use crate::util::io;
use crate::util::rpc::RpcServerBuilder;
use crate::{State, get_resource_file, process};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::Utc;
use daedalus as d;
use daedalus::minecraft::{LoggingSide, VersionInfo};
use daedalus::modded::{LoaderVersion, Manifest};
use regex::Regex;
use serde::Deserialize;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

#[cfg(target_os = "windows")]
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

mod args;

pub mod download;
pub mod jvm_args;
pub mod language;
pub mod optifine;
pub mod quick_play_version;

const UTF8_GAME_ARGUMENT_PREFIX: &str = "__THESEUS_UTF8__:";

fn encode_game_argument(argument: String) -> String {
    if argument.is_ascii() && !argument.starts_with(UTF8_GAME_ARGUMENT_PREFIX) {
        argument
    } else {
        format!(
            "{UTF8_GAME_ARGUMENT_PREFIX}{}",
            BASE64_STANDARD.encode(argument.as_bytes())
        )
    }
}

#[cfg(target_os = "windows")]
fn set_high_performance_gpu_preference(
    executable: impl AsRef<std::path::Path>,
) -> crate::Result<()> {
    const REGISTRY_PATH: &str =
        "Software\\Microsoft\\DirectX\\UserGpuPreferences";
    const REGISTRY_VALUE: &str = "GpuPreference=2;";

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = current_user.create_subkey(REGISTRY_PATH)?;
    let executable = executable.as_ref().to_string_lossy();

    if key
        .get_value::<String, _>(executable.as_ref())
        .ok()
        .as_deref()
        == Some(REGISTRY_VALUE)
    {
        return Ok(());
    }

    key.set_value(executable.as_ref(), &REGISTRY_VALUE)?;
    tracing::info!(%executable, "Set high-performance GPU preference");

    Ok(())
}

#[cfg(target_os = "linux")]
async fn high_performance_gpu_environment() -> Vec<(String, String)> {
    match switcheroo_gpu_environment().await {
        Ok(Some(environment)) => {
            tracing::info!(
                "Using switcheroo-control environment for high-performance GPU"
            );
            environment
        }
        Ok(None) => fallback_high_performance_gpu_environment(),
        Err(error) => {
            tracing::debug!(
                %error,
                "Failed to query switcheroo-control; using PRIME fallback"
            );
            fallback_high_performance_gpu_environment()
        }
    }
}

#[cfg(target_os = "linux")]
async fn switcheroo_gpu_environment()
-> crate::Result<Option<Vec<(String, String)>>> {
    use std::collections::HashMap;
    use zbus::zvariant::OwnedValue;
    use zbus::{Connection, Proxy};

    let connection = Connection::system().await?;
    let proxy = Proxy::new(
        &connection,
        "net.hadess.SwitcherooControl",
        "/net/hadess/SwitcherooControl",
        "net.hadess.SwitcherooControl",
    )
    .await?;
    let gpus: Vec<HashMap<String, OwnedValue>> =
        proxy.get_property("GPUs").await?;

    for mut gpu in gpus {
        let is_default = gpu
            .get("Default")
            .and_then(|value| bool::try_from(value).ok());
        if is_default != Some(false) {
            continue;
        }

        let Some(environment) = gpu
            .remove("Environment")
            .and_then(|value| Vec::<String>::try_from(value).ok())
        else {
            continue;
        };
        let mut entries = environment.chunks_exact(2);
        let parsed = entries
            .by_ref()
            .filter(|entry| !entry[0].is_empty())
            .map(|entry| (entry[0].clone(), entry[1].clone()))
            .collect::<Vec<_>>();
        if entries.remainder().is_empty() && !parsed.is_empty() {
            return Ok(Some(parsed));
        }
    }

    Ok(None)
}

#[cfg(target_os = "linux")]
fn fallback_high_performance_gpu_environment() -> Vec<(String, String)> {
    [
        ("DRI_PRIME", "1"),
        ("__NV_PRIME_RENDER_OFFLOAD", "1"),
        ("__VK_LAYER_NV_optimus", "NVIDIA_only"),
        ("__GLX_VENDOR_LIBRARY_NAME", "nvidia"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

// All nones -> disallowed
// 1+ true -> allowed
// 1+ false -> disallowed
#[tracing::instrument]
pub fn parse_rules(
    rules: &[d::minecraft::Rule],
    java_version: &str,
    quick_play_type: &QuickPlayType,
    minecraft_updated: bool,
) -> bool {
    if rules.is_empty() {
        return true;
    }

    let x = rules
        .iter()
        .map(|x| {
            parse_rule(x, java_version, quick_play_type, minecraft_updated)
        })
        .collect::<Vec<Option<bool>>>();

    !(x.iter().any(|x| x == &Some(false)) || x.iter().all(|x| x.is_none()))
}

// if anything is disallowed, it should NOT be included
// if anything is not disallowed, it shouldn't factor in final result
// if anything is not allowed, it should NOT be included
// if anything is allowed, it should be included
#[tracing::instrument]
pub fn parse_rule(
    rule: &d::minecraft::Rule,
    java_version: &str,
    quick_play_type: &QuickPlayType,
    minecraft_updated: bool,
) -> Option<bool> {
    use d::minecraft::{Rule, RuleAction};

    let res = match rule {
        Rule { os: Some(os), .. } => {
            crate::util::platform::os_rule(os, java_version, minecraft_updated)
        }
        Rule {
            features: Some(features),
            ..
        } => {
            !features.is_demo_user.unwrap_or(true)
                || features.has_custom_resolution.unwrap_or(false)
                || !features.has_quick_plays_support.unwrap_or(true)
                || (features.is_quick_play_singleplayer.unwrap_or(false)
                    && matches!(
                        quick_play_type,
                        QuickPlayType::Singleplayer(_)
                    ))
                || (features.is_quick_play_multiplayer.unwrap_or(false)
                    && matches!(quick_play_type, QuickPlayType::Server(..)))
                || !features.is_quick_play_realms.unwrap_or(true)
        }
        _ => match rule.action {
            RuleAction::Allow => return Some(true),
            RuleAction::Disallow => return Some(false),
        },
    };

    match rule.action {
        RuleAction::Allow => {
            if res {
                Some(true)
            } else {
                None
            }
        }
        RuleAction::Disallow => {
            if res {
                Some(false)
            } else {
                None
            }
        }
    }
}

macro_rules! processor_rules {
    ($dest:expr; $($name:literal : client => $client:expr, server => $server:expr;)+) => {
        $(std::collections::HashMap::insert(
            $dest,
            String::from($name),
            daedalus::modded::SidedDataEntry {
                client: String::from($client),
                server: String::from($server),
            },
        );)+
    }
}

fn processor_output_sha1(value: &str) -> Option<&str> {
    let value = value.trim().trim_matches(['\'', '"']);
    let value = value
        .strip_prefix("sha1:")
        .or_else(|| value.strip_prefix("SHA1:"))
        .unwrap_or(value);
    (value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then_some(value)
}

async fn processor_outputs_are_current(
    processor: &d::modded::Processor,
    libraries_dir: &Path,
    data: &std::collections::HashMap<String, d::modded::SidedDataEntry>,
    allowed_roots: &[&Path],
) -> bool {
    let inferred_outputs: std::collections::HashMap<String, String>;
    let outputs = if let Some(outputs) = processor
        .outputs
        .as_ref()
        .filter(|outputs| !outputs.is_empty())
    {
        outputs
    } else {
        let is_binary_patcher =
            processor.jar.split(':').nth(1).is_some_and(|artifact| {
                artifact.eq_ignore_ascii_case("binarypatcher")
            });
        if !is_binary_patcher {
            return false;
        }
        let mut inferred = std::collections::HashMap::new();
        for (index, argument) in processor.args.iter().enumerate() {
            if argument != "--output" {
                continue;
            }
            let Some(path_template) = processor.args.get(index + 1) else {
                return false;
            };
            let Some(key) = path_template
                .strip_prefix('{')
                .and_then(|key| key.strip_suffix('}'))
            else {
                return false;
            };
            let hash_key = format!("{key}_SHA");
            if !data.contains_key(&hash_key) {
                return false;
            }
            inferred.insert(path_template.clone(), format!("{{{hash_key}}}"));
        }
        if inferred.is_empty() {
            return false;
        }
        inferred_outputs = inferred;
        &inferred_outputs
    };
    let mut canonical_roots = Vec::with_capacity(allowed_roots.len());
    for root in allowed_roots {
        if let Ok(root) = tokio::fs::canonicalize(root).await {
            canonical_roots.push(root);
        }
    }
    if canonical_roots.is_empty() {
        return false;
    }

    for (path_template, hash_template) in outputs {
        let resolved = match args::get_processor_arguments(
            libraries_dir,
            &[path_template, hash_template],
            data,
        ) {
            Ok(resolved) => resolved,
            Err(error) => {
                tracing::debug!(
                    processor = %processor.jar,
                    %error,
                    "Could not resolve processor output"
                );
                return false;
            }
        };
        let [resolved_path, resolved_hash] = resolved.as_slice() else {
            return false;
        };
        let Some(expected_sha1) = processor_output_sha1(resolved_hash) else {
            return false;
        };
        let path = PathBuf::from(resolved_path);
        if !path.is_absolute() {
            return false;
        }
        let Ok(path) = tokio::fs::canonicalize(path).await else {
            return false;
        };
        if !canonical_roots.iter().any(|root| path.starts_with(root)) {
            return false;
        }
        let Ok((_, actual_sha1)) =
            crate::util::fetch::sha1_file_async(&path).await
        else {
            return false;
        };
        if !actual_sha1.eq_ignore_ascii_case(expected_sha1) {
            return false;
        }
    }

    true
}

pub async fn get_java_version_from_launch_context(
    context: &InstanceLaunchContext,
    version_info: &VersionInfo,
) -> crate::Result<Option<JavaVersion>> {
    if let Some(java) = context.launch_overrides.java_path.as_ref() {
        let java =
            crate::api::jre::check_jre(std::path::PathBuf::from(java)).await;

        if let Ok(java) = java {
            validate_loader_java_version(version_info, &java)?;
            return Ok(Some(java));
        }
    }

    let key = required_java_major(version_info);

    let java_version = crate::api::jre::find_java_for_version(key).await?;

    if let Some(java_version) = &java_version {
        validate_loader_java_version(version_info, java_version)?;
    }

    Ok(java_version)
}

fn required_java_major(version_info: &VersionInfo) -> u32 {
    if version_uses_liteloader(version_info) {
        8
    } else {
        version_info
            .java_version
            .as_ref()
            .map_or(8, |java| java.major_version)
    }
}

fn version_uses_liteloader(version_info: &VersionInfo) -> bool {
    version_info
        .libraries
        .iter()
        .any(|library| is_liteloader_library(&library.name))
}

fn is_liteloader_library(name: &str) -> bool {
    let mut coordinates = name.split(':');
    coordinates.next() == Some("com.mumfrey")
        && coordinates.next() == Some("liteloader")
        && coordinates.next().is_some()
}

fn validate_loader_java_version(
    version_info: &VersionInfo,
    java_version: &JavaVersion,
) -> crate::Result<()> {
    if version_uses_liteloader(version_info) && java_version.parsed_version != 8
    {
        return Err(crate::ErrorKind::LauncherError(format!(
            "LiteLoader requires Java 8, but Java {} is selected",
            java_version.parsed_version
        ))
        .into());
    }

    Ok(())
}

pub async fn get_loader_version_from_profile(
    game_version: &str,
    loader: ModLoader,
    loader_version: Option<&str>,
) -> crate::Result<Option<LoaderVersion>> {
    get_loader_version_from_profile_with_cache(
        game_version,
        loader,
        loader_version,
        None,
    )
    .await
}

pub async fn get_loader_version_from_profile_with_cache(
    game_version: &str,
    loader: ModLoader,
    loader_version: Option<&str>,
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Option<LoaderVersion>> {
    if loader == ModLoader::Vanilla {
        return Ok(None);
    }

    if loader == ModLoader::OptiFine {
        return optifine::resolve_loader_version(game_version, loader_version)
            .await;
    }

    let versions =
        crate::api::metadata::get_loader_versions_for_game_with_cache(
            loader.as_meta_str(),
            game_version,
            cache_behaviour,
        )
        .await?;

    if let Some(loaders) =
        loader_versions_for_game_version(&versions, game_version)
    {
        Ok(select_loader_version(loaders, loader_version).cloned())
    } else {
        Ok(None)
    }
}

fn select_loader_version<'a>(
    loaders: &'a [LoaderVersion],
    requested: Option<&str>,
) -> Option<&'a LoaderVersion> {
    let requested = requested.unwrap_or("latest");
    loaders
        .iter()
        .find(|version| match requested {
            "latest" => true,
            "stable" => version.stable,
            id => version.id == id,
        })
        .or_else(|| (requested == "stable").then(|| loaders.first()).flatten())
}

fn explicit_loader_version(requested: Option<&str>) -> Option<&str> {
    requested.filter(|version| {
        !version.is_empty() && !matches!(*version, "latest" | "stable")
    })
}

fn missing_loader_version_fallback(
    loader: ModLoader,
    game_version: &str,
    requested: Option<&str>,
) -> Result<&'static str, String> {
    if let Some(requested) = explicit_loader_version(requested) {
        Err(format!(
            "Loader version {requested} is not available for {} {game_version}",
            loader.as_str()
        ))
    } else {
        Ok("stable")
    }
}

fn installed_offline_loader_version(
    loader: ModLoader,
    requested: Option<&str>,
) -> Option<LoaderVersion> {
    if !matches!(
        loader,
        ModLoader::Fabric
            | ModLoader::Forge
            | ModLoader::NeoForge
            | ModLoader::Quilt
            | ModLoader::Cleanroom
            | ModLoader::LiteLoader
            | ModLoader::LegacyFabric
            | ModLoader::Babric
    ) {
        return None;
    }
    let id = explicit_loader_version(requested)?;
    Some(LoaderVersion {
        id: id.to_string(),
        url: String::new(),
        stable: false,
        profile_source: Default::default(),
        fallback_url: None,
    })
}

fn loader_versions_for_game_version<'a>(
    manifest: &'a Manifest,
    game_version: &str,
) -> Option<&'a [LoaderVersion]> {
    let version = manifest.game_versions.iter().find(|x| {
        x.id.replace(daedalus::modded::DUMMY_REPLACE_STRING, game_version)
            == game_version
    })?;

    if let Some(version_group) = &version.version_group {
        manifest
            .version_groups
            .iter()
            .find(|group| group.id == *version_group)
            .map(|group| group.loaders.as_slice())
    } else {
        Some(version.loaders.as_slice())
    }
}

/// Resolves the Minecraft version manifest and finds the index for the given
/// game version. If the version isn't found in the cache, forces a manifest
/// refresh to pick up newly-released versions.
pub async fn resolve_minecraft_manifest(
    game_version: &str,
    state: &State,
) -> crate::Result<(d::minecraft::VersionManifest, usize)> {
    resolve_minecraft_manifest_with_cache(game_version, state, None).await
}

pub async fn resolve_minecraft_manifest_with_cache(
    game_version: &str,
    state: &State,
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<(d::minecraft::VersionManifest, usize)> {
    let minecraft = crate::api::metadata::get_minecraft_versions_with_cache(
        cache_behaviour,
    )
    .await?;

    if let Some(idx) = minecraft
        .versions
        .iter()
        .position(|it| it.id == game_version)
    {
        return Ok((minecraft, idx));
    }

    // Version not found in the first manifest lookup. Online launches force a
    // refresh for newly released versions; offline launches repeat a cache-only
    // lookup so they never reach the network.
    let refreshed = crate::state::CachedEntry::get_minecraft_manifest(
        if cache_behaviour == Some(CacheBehaviour::CacheOnly) {
            Some(CacheBehaviour::CacheOnly)
        } else {
            Some(CacheBehaviour::MustRevalidate)
        },
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::NoValueFor("minecraft versions".to_string())
    })?;

    let idx = refreshed
        .versions
        .iter()
        .position(|it| it.id == game_version)
        .ok_or(crate::ErrorKind::LauncherError(format!(
            "Invalid game version: {game_version}"
        )))?;

    Ok((refreshed, idx))
}

async fn get_instance_full_path(
    instance_path: &str,
    game_dir_override: Option<&str>,
) -> crate::Result<PathBuf> {
    let state = State::get().await?;
    let full_path = io::canonicalize(
        state
            .directories
            .resolve_game_dir(instance_path, game_dir_override),
    )?;
    Ok(full_path)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstanceCompletionPolicy {
    FinalizeHere,
    DeferToInstallJob,
}

pub(crate) async fn run_instance_install_command(
    instance_id: String,
    cancellation: CancellationToken,
    mut command: Command,
) -> crate::Result<Output> {
    let state = State::get().await?;
    let instance_lock =
        state.lock_instance_content_exclusive(&instance_id).await;
    let task = tokio::spawn(async move {
        let _instance_lock = instance_lock;
        if cancellation.is_cancelled() {
            return Err(crate::ErrorKind::OtherError(
                "Install was canceled".to_string(),
            )
            .into());
        }
        command
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_task = tokio::spawn(async move {
            let mut output = Vec::new();
            if let Some(mut stdout) = stdout {
                stdout.read_to_end(&mut output).await?;
            }
            Ok::<_, std::io::Error>(output)
        });
        let stderr_task = tokio::spawn(async move {
            let mut output = Vec::new();
            if let Some(mut stderr) = stderr {
                stderr.read_to_end(&mut output).await?;
            }
            Ok::<_, std::io::Error>(output)
        });

        let (status, canceled) = tokio::select! {
            biased;
            _ = cancellation.cancelled() => {
                let _ = child.start_kill();
                (child.wait().await?, true)
            }
            status = child.wait() => (status?, false),
        };
        let stdout = stdout_task.await.map_err(std::io::Error::other)??;
        let stderr = stderr_task.await.map_err(std::io::Error::other)??;
        if canceled {
            return Err(crate::ErrorKind::OtherError(
                "Install was canceled".to_string(),
            )
            .into());
        }
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    });

    task.await?
}

pub async fn install_minecraft_with_reporter(
    context: &InstanceLaunchContext,
    repairing: bool,
    reporter: Option<InstallProgressReporter>,
    completion_policy: InstanceCompletionPolicy,
) -> crate::Result<()> {
    install_minecraft_with_local_source(
        context,
        repairing,
        reporter,
        None,
        completion_policy,
    )
    .await
}

async fn install_minecraft_with_local_source(
    context: &InstanceLaunchContext,
    repairing: bool,
    reporter: Option<InstallProgressReporter>,
    local_source: Option<&LocalRuntimeSource>,
    completion_policy: InstanceCompletionPolicy,
) -> crate::Result<()> {
    let instance = &context.instance;
    let content_set = &context.applied_content_set;
    let phase_details = InstallPhaseDetails::Minecraft {
        game_version: content_set.game_version.clone(),
        loader: content_set.loader,
    };
    let loading_bar = if reporter.is_none() {
        Some(
            init_loading(
                LoadingBarType::MinecraftDownload {
                    // If we are downloading minecraft for a profile, provide its name and uuid
                    instance_name: instance.name.clone(),
                    instance_id: instance.id.clone(),
                },
                100.0,
                "Downloading Minecraft",
            )
            .await?,
        )
    } else {
        None
    };

    let state = State::get().await?;

    crate::state::instances::commands::set_instance_install_stage(
        &instance.id,
        InstanceInstallStage::MinecraftInstalling,
        &state.pool,
    )
    .await?;
    emit_instance(&instance.id, InstancePayloadType::Edited).await?;

    let instance_path = get_instance_full_path(
        &instance.path,
        instance.game_dir_override.as_deref(),
    )
    .await?;
    if let Some(reporter) = &reporter {
        reporter
            .update(
                InstallPhaseId::ResolvingMinecraft,
                None,
                phase_details.clone(),
            )
            .await?;
    }
    let (minecraft, version_index) =
        resolve_minecraft_manifest(&content_set.game_version, &state).await?;
    let version = &minecraft.versions[version_index];
    let minecraft_updated = version_index
        <= minecraft
            .versions
            .iter()
            .position(|x| x.id == "22w16a")
            .unwrap_or(0);

    if content_set.loader != ModLoader::Vanilla
        && let Some(reporter) = &reporter
    {
        reporter
            .update(
                InstallPhaseId::ResolvingLoader,
                None,
                phase_details.clone(),
            )
            .await?;
    }

    let mut loader_version = get_loader_version_from_profile(
        &content_set.game_version,
        content_set.loader,
        content_set.loader_version.as_deref(),
    )
    .await?;

    // If no loader version is selected, try to select the stable version!
    if content_set.loader != ModLoader::Vanilla && loader_version.is_none() {
        let fallback = missing_loader_version_fallback(
            content_set.loader,
            &content_set.game_version,
            content_set.loader_version.as_deref(),
        )
        .map_err(crate::ErrorKind::LauncherError)?;
        loader_version = get_loader_version_from_profile(
            &content_set.game_version,
            content_set.loader,
            Some(fallback),
        )
        .await?;

        crate::state::instances::commands::set_applied_content_set_loader_version(
            &instance.id,
            loader_version.as_ref().map(|x| x.id.as_str()),
            &state.pool,
        )
        .await?;
    }

    let version_jar =
        loader_version.as_ref().map_or(version.id.clone(), |it| {
            format!("{}-{}", version.id.clone(), it.id.clone())
        });

    // Download version info (5)
    let mut version_info = download::download_version_info(
        &state,
        version,
        content_set.loader,
        loader_version.as_ref(),
        Some(repairing),
        loading_bar.as_ref(),
        reporter.as_ref(),
    )
    .await?;

    let key = required_java_major(&version_info);
    if let Some(reporter) = &reporter {
        reporter
            .update(
                InstallPhaseId::PreparingJava,
                Some(InstallProgress {
                    current: 0,
                    total: 4,
                    secondary: None,
                }),
                InstallPhaseDetails::Java {
                    major_version: key,
                    step: InstallJavaStep::Resolving,
                },
            )
            .await?;
    }
    let java_installation = if let Some(java_version) =
        get_java_version_from_launch_context(context, &version_info).await?
    {
        Some((std::path::PathBuf::from(java_version.path), false))
    } else if let Some(discovered) = crate::api::jre::find_java_for_version(key)
        .await
        .unwrap_or_default()
    {
        tracing::info!(
            "Reusing discovered Java {} at {}",
            discovered.version,
            discovered.path
        );
        Some((std::path::PathBuf::from(discovered.path), true))
    } else {
        if let Some(reporter) = &reporter {
            crate::api::jre::auto_install_java_with_reporter(
                key,
                reporter.clone(),
            )
            .await?
        } else {
            crate::api::jre::auto_install_java_with_loading(key, true).await?
        }
        .map(|path| (path, true))
    };

    let java_version = if let Some((java_path, set_java)) = java_installation {
        if let Some(reporter) = &reporter {
            reporter
                .update(
                    InstallPhaseId::PreparingJava,
                    Some(InstallProgress {
                        current: 4,
                        total: 4,
                        secondary: None,
                    }),
                    InstallPhaseDetails::Java {
                        major_version: key,
                        step: InstallJavaStep::Validating,
                    },
                )
                .await?;
        }
        let java_version = crate::api::jre::check_jre(java_path).await?;
        validate_loader_java_version(&version_info, &java_version)?;

        if set_java {
            java_version.upsert(&state.pool).await?;
        }

        Some(java_version)
    } else {
        None
    };

    // Download minecraft (5-90)
    if let Some(reporter) = &reporter {
        reporter
            .update(
                InstallPhaseId::DownloadingMinecraft,
                None,
                phase_details.clone(),
            )
            .await?;
    }
    download::download_minecraft(
        &state,
        local_source,
        &content_set.game_version,
        &version_info,
        loading_bar.as_ref(),
        java_version
            .as_ref()
            .map(|java| java.architecture.as_str())
            .unwrap_or(std::env::consts::ARCH),
        repairing,
        minecraft_updated,
        reporter.clone(),
        phase_details.clone(),
    )
    .await?;

    let client_path = state
        .directories
        .version_dir(&version_jar)
        .join(format!("{version_jar}.jar"));

    let Some(java_version) = java_version else {
        let protocol_version =
            read_protocol_version_from_jar(client_path).await?;
        crate::state::instances::commands::set_applied_content_set_protocol_version(
            &instance.id,
            protocol_version,
            &state.pool,
        )
        .await?;
        if completion_policy == InstanceCompletionPolicy::FinalizeHere {
            crate::state::instances::commands::set_instance_install_stage(
                &instance.id,
                InstanceInstallStage::Installed,
                &state.pool,
            )
            .await?;
            emit_instance(&instance.id, InstancePayloadType::Edited).await?;
        }
        if let Some(loading_bar) = &loading_bar {
            emit_loading(
                loading_bar,
                1.0,
                Some("Finished downloading Minecraft resources"),
            )?;
        }
        tracing::info!(
            java_version = key,
            instance_id = instance.id,
            "Postponed Java setup after downloading Minecraft resources"
        );
        return Ok(());
    };

    if content_set.loader == ModLoader::OptiFine
        && let Some(loader_version) = &loader_version
    {
        if let Some(reporter) = &reporter {
            reporter
                .update(
                    InstallPhaseId::RunningLoaderProcessors,
                    None,
                    phase_details.clone(),
                )
                .await?;
        }
        optifine::install_optifine_libraries(
            &state,
            std::path::Path::new(&java_version.path),
            &content_set.game_version,
            &loader_version.id,
            &client_path,
        )
        .await?;
    }

    if let Some(processors) = &version_info.processors {
        let libraries_dir = state.directories.libraries_dir();

        if let Some(ref mut data) = version_info.data {
            processor_rules! {
                data;
                "SIDE":
                    client => "client",
                    server => "";
                "MINECRAFT_JAR" :
                    client => client_path.to_string_lossy(),
                    server => "";
                "MINECRAFT_VERSION":
                    client => content_set.game_version.clone(),
                    server => "";
                "ROOT":
                    client => instance_path.to_string_lossy(),
                    server => "";
                "LIBRARY_DIR":
                    client => libraries_dir.to_string_lossy(),
                    server => "";
            }

            if let Some(loading_bar) = &loading_bar {
                emit_loading(
                    loading_bar,
                    0.0,
                    Some("Running forge processors"),
                )?;
            }
            let total_length = processors.len();
            if let Some(reporter) = &reporter {
                reporter
                    .update(
                        InstallPhaseId::RunningLoaderProcessors,
                        Some(InstallProgress {
                            current: 0,
                            total: total_length as u64,
                            secondary: None,
                        }),
                        phase_details.clone(),
                    )
                    .await?;
            }

            // Forge processors (90-100)
            for (index, processor) in processors.iter().enumerate() {
                let processor_started = Instant::now();
                if let Some(sides) = &processor.sides
                    && !sides.contains(&String::from("client"))
                {
                    if let Some(reporter) = &reporter {
                        reporter
                            .update(
                                InstallPhaseId::RunningLoaderProcessors,
                                Some(InstallProgress {
                                    current: (index + 1) as u64,
                                    total: total_length as u64,
                                    secondary: None,
                                }),
                                phase_details.clone(),
                            )
                            .await?;
                    }
                    continue;
                }
                if processor_outputs_are_current(
                    processor,
                    &libraries_dir,
                    data,
                    &[&instance_path, &libraries_dir],
                )
                .await
                {
                    tracing::info!(
                        processor = %processor.jar,
                        index,
                        duration_ms = processor_started.elapsed().as_millis(),
                        "Skipped Forge processor because all declared outputs are valid"
                    );
                    if let Some(loading_bar) = &loading_bar {
                        emit_loading(
                            loading_bar,
                            30.0 / total_length as f64,
                            Some(&format!(
                                "Running forge processor {index}/{total_length}"
                            )),
                        )?;
                    }
                    if let Some(reporter) = &reporter {
                        reporter
                            .update(
                                InstallPhaseId::RunningLoaderProcessors,
                                Some(InstallProgress {
                                    current: (index + 1) as u64,
                                    total: total_length as u64,
                                    secondary: None,
                                }),
                                phase_details.clone(),
                            )
                            .await?;
                    }
                    continue;
                }

                let cp = {
                    let mut cp = processor.classpath.clone();
                    cp.push(processor.jar.clone());
                    cp
                };

                let mut command = Command::new(&java_version.path);
                command
                    .arg("-cp")
                    .arg(args::get_class_paths_jar(
                        &libraries_dir,
                        &cp,
                        &java_version.architecture,
                    )?)
                    .arg(
                        args::get_processor_main_class(args::get_lib_path(
                            &libraries_dir,
                            &processor.jar,
                            false,
                        )?)
                        .await?
                        .ok_or_else(|| {
                            crate::ErrorKind::LauncherError(format!(
                                "Could not find processor main class for {}",
                                processor.jar
                            ))
                        })?,
                    )
                    .args(args::get_processor_arguments(
                        &libraries_dir,
                        &processor.args,
                        data,
                    )?);
                let child = run_instance_install_command(
                    instance.id.clone(),
                    reporter
                        .as_ref()
                        .map(InstallProgressReporter::cancellation_token)
                        .unwrap_or_default(),
                    command,
                )
                .await
                .map_err(|err| {
                    crate::ErrorKind::LauncherError(format!(
                        "Error running processor: {err}",
                    ))
                })?;

                if !child.status.success() {
                    return Err(crate::ErrorKind::LauncherError(format!(
                        "Processor error: {}",
                        String::from_utf8_lossy(&child.stderr)
                    ))
                    .as_error());
                }
                tracing::info!(
                    processor = %processor.jar,
                    index,
                    duration_ms = processor_started.elapsed().as_millis(),
                    "Completed Forge processor"
                );

                if let Some(loading_bar) = &loading_bar {
                    emit_loading(
                        loading_bar,
                        30.0 / total_length as f64,
                        Some(&format!(
                            "Running forge processor {index}/{total_length}"
                        )),
                    )?;
                }
                if let Some(reporter) = &reporter {
                    reporter
                        .update(
                            InstallPhaseId::RunningLoaderProcessors,
                            Some(InstallProgress {
                                current: (index + 1) as u64,
                                total: total_length as u64,
                                secondary: None,
                            }),
                            phase_details.clone(),
                        )
                        .await?;
                }
            }
        }
    }

    let protocol_version = read_protocol_version_from_jar(client_path).await?;

    crate::state::instances::commands::set_applied_content_set_protocol_version(
        &instance.id,
        protocol_version,
        &state.pool,
    )
    .await?;
    if completion_policy == InstanceCompletionPolicy::FinalizeHere {
        crate::state::instances::commands::set_instance_install_stage(
            &instance.id,
            InstanceInstallStage::Installed,
            &state.pool,
        )
        .await?;
        emit_instance(&instance.id, InstancePayloadType::Edited).await?;
    }
    if let Some(loading_bar) = &loading_bar {
        emit_loading(loading_bar, 1.0, Some("Finished installing"))?;
    }

    Ok(())
}

pub async fn install_minecraft_for_instance_id_with_reporter(
    instance_id: &str,
    repairing: bool,
    reporter: Option<InstallProgressReporter>,
    completion_policy: InstanceCompletionPolicy,
) -> crate::Result<()> {
    let state = State::get().await?;
    let context =
        crate::state::instances::commands::get_instance_launch_context(
            instance_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Tried to install a nonexistent or unloaded instance {instance_id}!"
            ))
        })?;

    install_minecraft_with_reporter(
        &context,
        repairing,
        reporter,
        completion_policy,
    )
    .await
}

pub async fn install_minecraft_for_instance_id_with_local_source(
    instance_id: &str,
    local_source: Option<LocalRuntimeSource>,
    repairing: bool,
    reporter: Option<InstallProgressReporter>,
    completion_policy: InstanceCompletionPolicy,
) -> crate::Result<()> {
    let state = State::get().await?;
    let context =
        crate::state::instances::commands::get_instance_launch_context(
            instance_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Tried to install a nonexistent or unloaded instance {instance_id}!"
            ))
        })?;

    install_minecraft_with_local_source(
        &context,
        repairing,
        reporter,
        local_source.as_ref(),
        completion_policy,
    )
    .await
}

pub async fn read_protocol_version_from_jar(
    path: PathBuf,
) -> crate::Result<Option<u32>> {
    let zip = async_zip::tokio::read::fs::ZipFileReader::new(path).await?;
    let Some(entry_index) = zip
        .file()
        .entries()
        .iter()
        .position(|x| matches!(x.filename().as_str(), Ok("version.json")))
    else {
        return Ok(None);
    };

    #[derive(Deserialize, Debug)]
    struct VersionData {
        protocol_version: Option<u32>,
    }

    let mut data = vec![];
    zip.reader_with_entry(entry_index)
        .await?
        .read_to_end_checked(&mut data)
        .await?;
    let data: VersionData = serde_json::from_slice(&data)?;

    Ok(data.protocol_version)
}

fn link_project_and_version(
    link: &InstanceLink,
) -> (Option<&String>, Option<&String>) {
    match link {
        InstanceLink::ModrinthModpack {
            project_id,
            version_id,
        }
        | InstanceLink::CurseForgeModpack {
            project_id,
            version_id,
        } => (Some(project_id), Some(version_id)),
        InstanceLink::ServerProject { project_id } => (Some(project_id), None),
        InstanceLink::ServerProjectModpack {
            server_project_id,
            content_version_id,
            ..
        } => (Some(server_project_id), Some(content_version_id)),
        InstanceLink::ImportedModpack {
            project_id,
            version_id,
            ..
        } => (project_id.as_ref(), version_id.as_ref()),
        InstanceLink::Unmanaged | InstanceLink::SharedInstance { .. } => {
            (None, None)
        }
    }
}

#[tracing::instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn launch_minecraft(
    java_args: &[String],
    env_args: &[(String, String)],
    mc_set_options: &[(String, String)],
    wrapper: &Option<String>,
    memory: &MemorySettings,
    resolution: &WindowSize,
    credentials: &Credentials,
    post_exit_hook: Option<String>,
    context: &InstanceLaunchContext,
    gc_intent: Option<crate::launcher::jvm_args::GcLaunchIntent>,
    gc_report: &mut Option<crate::launcher::jvm_args::GcLaunchReport>,
    mut quick_play_type: QuickPlayType,
    offline_mode: bool,
) -> crate::Result<ProcessMetadata> {
    let instance = &context.instance;
    let content_set = &context.applied_content_set;

    if instance.install_stage == InstanceInstallStage::PackInstalling
        || instance.install_stage == InstanceInstallStage::MinecraftInstalling
    {
        return Err(crate::ErrorKind::LauncherError(
            "Instance is still installing".to_string(),
        )
        .into());
    }

    if instance.install_stage != InstanceInstallStage::Installed {
        return Err(crate::ErrorKind::LauncherError(
            "Instance is not installed; start an install job first".to_string(),
        )
        .into());
    }

    let state = State::get().await?;

    let instance_path = get_instance_full_path(
        &instance.path,
        instance.game_dir_override.as_deref(),
    )
    .await?;
    let offline_skin_pack =
        crate::minecraft_skins::prepare_offline_skin_resource_pack(
            credentials,
            &instance_path,
            &content_set.game_version,
        )
        .await?;

    let cache_behaviour = offline_mode.then_some(CacheBehaviour::CacheOnly);
    let (minecraft, version_index) = resolve_minecraft_manifest_with_cache(
        &content_set.game_version,
        &state,
        cache_behaviour,
    )
    .await?;
    let version = &minecraft.versions[version_index];
    let minecraft_updated = version_index
        <= minecraft
            .versions
            .iter()
            .position(|x| x.id == "22w16a")
            .unwrap_or(0);

    let loader_version = if offline_mode {
        if let Some(loader_version) = installed_offline_loader_version(
            content_set.loader,
            content_set.loader_version.as_deref(),
        ) {
            Some(loader_version)
        } else {
            get_loader_version_from_profile_with_cache(
                &content_set.game_version,
                content_set.loader,
                content_set.loader_version.as_deref(),
                cache_behaviour,
            )
            .await?
        }
    } else {
        get_loader_version_from_profile_with_cache(
            &content_set.game_version,
            content_set.loader,
            content_set.loader_version.as_deref(),
            cache_behaviour,
        )
        .await?
    };

    if content_set.loader != ModLoader::Vanilla && loader_version.is_none() {
        return Err(crate::ErrorKind::LauncherError(format!(
            "No loader version selected for {}",
            content_set.loader.as_str()
        ))
        .into());
    }

    let version_jar =
        loader_version.as_ref().map_or(version.id.clone(), |it| {
            format!("{}-{}", version.id.clone(), it.id.clone())
        });

    let mut version_info = if offline_mode {
        download::load_local_version_info(
            &state,
            version,
            content_set.loader,
            loader_version.as_ref(),
        )
        .await?
    } else {
        download::download_version_info(
            &state,
            version,
            content_set.loader,
            loader_version.as_ref(),
            None,
            None,
            None,
        )
        .await?
    };
    if version_info.logging.is_none() {
        let requires_logging_info = version_index
            <= minecraft
                .versions
                .iter()
                .position(|x| x.id == "13w39a")
                .unwrap_or(0);
        if requires_logging_info && !offline_mode {
            version_info = download::download_version_info(
                &state,
                version,
                content_set.loader,
                loader_version.as_ref(),
                Some(true),
                None,
                None,
            )
            .await?;
        }
    }

    if offline_mode {
        download::ensure_local_log_config(&state, &version_info)?;
    } else {
        let _ =
            download_log_config(&state, None, &version_info, None, false, None)
                .await?;
    }

    let java_version = if let Some(java) =
        context.launch_overrides.java_path.as_ref()
    {
        crate::api::jre::check_jre(std::path::PathBuf::from(java)).await?
    } else {
        let key = required_java_major(&version_info);

        if let Some(java) = crate::api::jre::find_java_for_version(key).await? {
            java
        } else if let Some(java) =
            crate::api::jre::find_compatible_java_for_version(key).await?
        {
            tracing::info!(
                version = java.version,
                java = java.path,
                "Using a compatible Java runtime instead of the recommended version"
            );
            java
        } else {
            return Err(crate::ErrorKind::LauncherError(
                "Missing correct java installation".to_string(),
            )
            .into());
        }
    };

    // Test jre version
    let java_version =
        crate::api::jre::check_jre(java_version.path.clone().into()).await?;
    validate_loader_java_version(&version_info, &java_version)?;

    // Runtime-verify and fall back for GC arguments against the *actual* JVM
    // that will run Minecraft. The frontend supplies an ordered candidate
    // chain; we keep the preferred strategy only if this JVM understands it,
    // pruning unsupported tuning flags and falling back down the chain as
    // needed. A `None` keeps the args passed in untouched.
    let mut resolved_java_args = java_args.to_vec();
    if let Some(gc_intent) = gc_intent {
        tracing::info!(
            java = %java_version.path,
            "Verifying GC arguments against the selected JVM",
        );
        let report = crate::launcher::jvm_args::resolve_gc_block(
            Path::new(java_version.path.as_str()),
            &mut resolved_java_args,
            &gc_intent,
        )
        .await;
        if report.fell_back() {
            tracing::info!(?report, "GC arguments adjusted for this JVM");
        }
        *gc_report = Some(report);
    }

    let settings = crate::state::Settings::get(&state.pool).await?;

    #[cfg(target_os = "windows")]
    if settings.auto_set_java_high_performance_mode {
        if let Err(error) =
            set_high_performance_gpu_preference(&java_version.path)
        {
            tracing::warn!(%error, java = %java_version.path, "Failed to set Java high-performance GPU preference");
        }

        match std::env::current_exe() {
            Ok(launcher_path) => {
                if let Err(error) =
                    set_high_performance_gpu_preference(&launcher_path)
                {
                    tracing::warn!(%error, launcher = %launcher_path.display(), "Failed to set launcher high-performance GPU preference");
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Failed to determine launcher executable for high-performance GPU preference");
            }
        }
    }

    #[cfg(target_os = "linux")]
    let high_performance_gpu_environment =
        if settings.auto_set_java_high_performance_mode {
            high_performance_gpu_environment().await
        } else {
            Vec::new()
        };

    let vanilla_client_path = state
        .directories
        .version_dir(&version_jar)
        .join(format!("{version_jar}.jar"));
    let client_path = match crate::api::instance::assemble_for_launch(
        &instance_path,
        &content_set.game_version,
        &vanilla_client_path,
    )
    .await?
    {
        Some(assembled) => assembled,
        None => vanilla_client_path,
    };

    let args = version_info.arguments.clone().unwrap_or_default();
    let mut command = match wrapper {
        Some(hook) => {
            let mut cmd = shlex::split(hook)
                .ok_or_else(|| {
                    crate::ErrorKind::LauncherError(format!(
                        "Invalid wrapper command: {hook}",
                    ))
                })?
                .into_iter();
            let mut command = Command::new(cmd.next().ok_or(
                crate::ErrorKind::LauncherError(
                    "Empty wrapper command".to_owned(),
                ),
            )?);
            command.args(cmd);
            command.arg(&java_version.path);
            command
        }
        None => Command::new(&java_version.path),
    };

    let env_args = Vec::from(env_args);

    // Check if instance has a running process, and reject running the command if it does
    // Done late so a quick double call doesn't launch two instances
    let existing_processes = process::get_by_instance_id(&instance.id).await?;
    if let Some(process) = existing_processes.first() {
        return Err(crate::ErrorKind::LauncherError(format!(
            "Instance {} is already running as process {}",
            instance.id, process.uuid
        ))
        .as_error());
    }

    let natives_dir = state.directories.version_natives_dir(&version_jar);
    if !natives_dir.exists() {
        io::create_dir_all(&natives_dir).await?;
    }
    download::ensure_native_libraries_extracted(
        &state.directories.natives_dir(),
        &state.directories.libraries_dir(),
        &state.directories.caches_dir(),
        version_info.libraries.as_slice(),
        &version_jar,
        &java_version.architecture,
        minecraft_updated,
    )
    .await?;

    let quick_play_version =
        QuickPlayVersion::find_version(version_index, &minecraft.versions);
    tracing::debug!(
        "Found QuickPlayVersion for {}: {quick_play_version:?}",
        content_set.game_version
    );
    if let QuickPlayType::Server(address) = &mut quick_play_type
        && quick_play_version.server >= QuickPlayServerVersion::BuiltinLegacy
    {
        // Record last-played for the original server address immediately so
        // recent-worlds can match without DNS/SRV resolution.
        let original = match address {
            ServerAddress::Unresolved(address) => parse_server_address(address)
                .ok()
                .map(|(h, p)| (h.to_owned(), p)),
            ServerAddress::Resolved {
                original_host,
                original_port,
                ..
            } => Some((original_host.clone(), *original_port)),
        };
        if let Some((host, port)) = original
            && let Err(e) = (JoinLogEntry {
                instance_id: instance.id.clone(),
                host,
                port,
                join_time: Utc::now(),
            })
            .upsert(&state.pool)
            .await
        {
            tracing::warn!("Failed to write server join log entry: {e}");
        }

        address.resolve().await?;
    }

    let (main_class_keep_alive, main_class_path) =
        get_resource_file!(env "JAVA_JARS_DIR" / "theseus.jar")?;
    let yggdrasil_agent = if credentials.is_yggdrasil() {
        let account = credentials.yggdrasil.as_ref().ok_or_else(|| {
            crate::ErrorKind::LauncherError(
                "Yggdrasil account metadata is missing".to_string(),
            )
            .as_error()
        })?;
        let metadata =
            crate::state::fetch_yggdrasil_metadata(&account.api_root).await?;
        let agent_path =
            main_class_keep_alive.path().join("authlib-injector.jar");
        io::write(
            &agent_path,
            include_bytes!(concat!(
                env!("JAVA_JARS_DIR"),
                "/authlib-injector.jar"
            )),
        )
        .await?;
        Some((
            io::canonicalize(agent_path)?,
            metadata.api_root,
            BASE64_STANDARD.encode(metadata.raw),
        ))
    } else {
        None
    };

    let rpc_server = RpcServerBuilder::new().launch().await?;

    command.args(
        args::get_jvm_arguments(
            args.get(&d::minecraft::ArgumentType::Jvm)
                .map(|x| x.as_slice()),
            &natives_dir,
            &state.directories.libraries_dir(),
            &state.directories.log_configs_dir(),
            &args::get_class_paths(
                &state.directories.libraries_dir(),
                version_info.libraries.as_slice(),
                &[&main_class_path, &client_path],
                &java_version.architecture,
                minecraft_updated,
            )?,
            &main_class_path,
            &version_jar,
            *memory,
            resolved_java_args.clone(),
            &java_version.architecture,
            &quick_play_type,
            quick_play_version,
            version_info
                .logging
                .as_ref()
                .and_then(|x| x.get(&LoggingSide::Client)),
            rpc_server.address(),
        )?
        .into_iter(),
    );

    // The java launcher requires access to java.lang.reflect in order to force access in to
    // whatever module the main class is in
    if java_version.parsed_version >= 9 {
        command.arg("--add-opens=java.base/java.lang.reflect=ALL-UNNAMED");
    }

    // The java launcher code requires internal JDK code in Java 25+ in order to support JEP 512
    if java_version.parsed_version >= 25 {
        command.arg("--add-opens=java.base/jdk.internal.misc=ALL-UNNAMED");
    }

    if let Some((agent_path, api_root, prefetched_metadata)) = yggdrasil_agent {
        command
            .arg(format!(
                "-javaagent:{}={api_root}",
                agent_path.to_string_lossy()
            ))
            .arg("-Dauthlibinjector.side=client")
            .arg(format!(
                "-Dauthlibinjector.yggdrasil.prefetched={prefetched_metadata}"
            ));
    }

    command
        .arg("com.modrinth.theseus.MinecraftLaunch")
        .arg(version_info.main_class.clone())
        .args(
            args::get_minecraft_arguments(
                args.get(&d::minecraft::ArgumentType::Game)
                    .map(|x| x.as_slice()),
                version_info.minecraft_arguments.as_deref(),
                credentials,
                &version.id,
                &version_info.asset_index.id,
                &instance_path,
                &state.directories.assets_dir(),
                &version.type_,
                *resolution,
                &java_version.architecture,
                &quick_play_type,
                quick_play_version,
            )
            .await?
            .into_iter()
            .map(encode_game_argument),
        )
        .current_dir(instance_path.clone());

    // CARGO-set DYLD_LIBRARY_PATH breaks Minecraft on macOS during testing on playground
    #[cfg(target_os = "macos")]
    if std::env::var("CARGO").is_ok() {
        command.env_remove("DYLD_FALLBACK_LIBRARY_PATH");
    }
    // Java options should be set in instance options (the existence of _JAVA_OPTIONS overwrites them)
    command.env_remove("_JAVA_OPTIONS");

    command.envs(env_args);

    #[cfg(target_os = "linux")]
    command.envs(high_performance_gpu_environment);

    // Overwrites the minecraft options.txt file with the settings from the profile
    // Uses 'a:b' syntax which is not quite yaml
    let options_path = instance_path.join("options.txt");
    let options_existed = options_path.exists();

    if !mc_set_options.is_empty()
        || offline_skin_pack.enabled_pack_id.is_some()
        || options_existed
        || !settings.locale.is_empty()
    {
        let (mut options_string, input_encoding) = if options_existed {
            io::read_any_encoding_to_string(&options_path).await?
        } else {
            (String::new(), encoding_rs::UTF_8)
        };

        // UTF-16 encodings may be successfully detected and read, but we cannot encode
        // them back, and it's technically possible that the game client strongly expects
        // such encoding
        if input_encoding != input_encoding.output_encoding() {
            return Err(crate::ErrorKind::LauncherError(format!(
                "The instance options.txt file uses an unsupported encoding: {}. \
                Please either turn off instance options that need to modify this file, \
                or convert the file to an encoding that both the game and this app support, \
                such as UTF-8.",
                input_encoding.name()
            ))
            .into());
        }

        let language_options = language::game_language_options(
            &settings.locale,
            version.release_time,
            &options_string,
            instance_path.join("saves").exists(),
        );

        if !mc_set_options.is_empty()
            || !language_options.is_empty()
            || offline_skin_pack.enabled_pack_id.is_some()
            || options_existed
        {
            for (key, value) in
                mc_set_options.iter().chain(language_options.iter())
            {
                let re =
                    Regex::new(&format!(r"(?m)^{}:.*$", regex::escape(key)))?;
                // check if the regex exists in the file
                if !re.is_match(&options_string) {
                    // The key was not found in the file, so append it
                    write!(&mut options_string, "\n{key}:{value}").unwrap();
                } else {
                    let replaced_string = re
                        .replace_all(&options_string, &format!("{key}:{value}"))
                        .to_string();
                    options_string = replaced_string;
                }
            }

            update_offline_skin_resource_pack_option(
                &mut options_string,
                offline_skin_pack,
            )?;

            io::write(&options_path, input_encoding.encode(&options_string).0)
                .await?;
        }
    }

    crate::state::instances::commands::set_instance_last_played(
        &instance.id,
        Utc::now(),
        &state.pool,
    )
    .await?;

    let _ = state
        .discord_rpc
        .set_activity(&format!("Playing {}", instance.name), true)
        .await;

    // Create Minecraft child by inserting it into the state
    // This also spawns the process and prepares the subsequent processes
    state
        .process_manager
        .insert_new_process(
            &instance.id,
            &instance.path,
            &instance.name,
            command,
            post_exit_hook,
            state
                .directories
                .game_logs_dir(&state.directories.instance_game_dir(&instance)),
            version_info.logging.is_some(),
            main_class_keep_alive,
            rpc_server,
            async |process: &ProcessMetadata, rpc_server| {
                let process_start_time = process.start_time.to_rfc3339();
                let instance_created_time = instance.created.to_rfc3339();
                let instance_modified_time = instance.modified.to_rfc3339();
                let (link_project_id, link_version_id) =
                    link_project_and_version(&context.link);
                let system_properties = [
                    ("modrinth.process.startTime", Some(&process_start_time)),
                    ("modrinth.profile.created", Some(&instance_created_time)),
                    ("modrinth.profile.icon", instance.icon_path.as_ref()),
                    ("modrinth.profile.link.project", link_project_id),
                    ("modrinth.profile.link.version", link_version_id),
                    (
                        "modrinth.profile.modified",
                        Some(&instance_modified_time),
                    ),
                    ("modrinth.profile.name", Some(&instance.name)),
                ];
                for (key, value) in system_properties {
                    let Some(value) = value else {
                        continue;
                    };
                    rpc_server
                        .call_method_2::<()>("set_system_property", key, value)
                        .await?;
                }
                rpc_server.call_method::<()>("launch").await?;
                Ok(())
            },
        )
        .await
}

#[cfg(test)]
mod game_argument_encoding_tests {
    use super::*;

    #[test]
    fn ascii_game_argument_stays_unchanged() {
        assert_eq!(
            encode_game_argument("--username".to_string()),
            "--username"
        );
    }

    #[test]
    fn non_ascii_game_argument_uses_ascii_transport() {
        let original = r"E:\Games\Minecraft\profiles\Prominence™ II";
        let encoded = encode_game_argument(original.to_string());

        assert!(encoded.is_ascii());
        let payload = encoded
            .strip_prefix(UTF8_GAME_ARGUMENT_PREFIX)
            .expect("non-ASCII argument should be encoded");
        assert_eq!(
            BASE64_STANDARD.decode(payload).unwrap(),
            original.as_bytes()
        );
    }

    #[test]
    fn reserved_prefix_is_escaped() {
        let original = format!("{UTF8_GAME_ARGUMENT_PREFIX}literal");
        let encoded = encode_game_argument(original.clone());
        let payload = encoded.strip_prefix(UTF8_GAME_ARGUMENT_PREFIX).unwrap();

        assert_eq!(
            BASE64_STANDARD.decode(payload).unwrap(),
            original.as_bytes()
        );
    }
}

fn update_offline_skin_resource_pack_option(
    options_string: &mut String,
    offline_skin_pack: crate::minecraft_skins::OfflineSkinPackOptions,
) -> crate::Result<()> {
    use crate::minecraft_skins::{
        OFFLINE_SKIN_PACK_LEGACY_ID, OFFLINE_SKIN_PACK_MODERN_ID,
    };

    let resource_packs = Regex::new(r"(?m)^resourcePacks:(.*)$")?;
    let existing_value = resource_packs
        .captures(options_string)
        .and_then(|captures| captures.get(1))
        .map_or("[]", |value| value.as_str().trim());
    if offline_skin_pack.enabled_pack_id.is_none()
        && !existing_value.contains(OFFLINE_SKIN_PACK_LEGACY_ID)
        && !existing_value.contains(OFFLINE_SKIN_PACK_MODERN_ID)
    {
        return Ok(());
    }
    let Ok(mut packs) = serde_json::from_str::<Vec<String>>(existing_value)
    else {
        tracing::warn!(
            "Skipping offline skin resource-pack option update because resourcePacks in options.txt is malformed"
        );
        return Ok(());
    };

    packs.retain(|pack| {
        pack != OFFLINE_SKIN_PACK_LEGACY_ID
            && pack != OFFLINE_SKIN_PACK_MODERN_ID
    });
    if let Some(pack_id) = offline_skin_pack.enabled_pack_id
        && !packs.iter().any(|pack| pack == pack_id)
    {
        if pack_id == OFFLINE_SKIN_PACK_MODERN_ID
            && !packs.iter().any(|pack| pack == "vanilla")
        {
            packs.insert(0, "vanilla".to_string());
        }
        packs.push(pack_id.to_string());
    }

    let value = serde_json::to_string(&packs)?;
    if resource_packs.is_match(options_string) {
        *options_string = resource_packs
            .replace_all(options_string, format!("resourcePacks:{value}"))
            .to_string();
    } else if options_string.is_empty() {
        write!(options_string, "resourcePacks:{value}").unwrap();
    } else {
        write!(options_string, "\nresourcePacks:{value}").unwrap();
    }

    Ok(())
}

#[cfg(test)]
mod loader_resolution_tests {
    use super::*;
    use daedalus::minecraft::RuleAction;

    #[test]
    fn disallow_only_library_rules_are_not_downloaded() {
        let rules = [d::minecraft::Rule {
            action: RuleAction::Disallow,
            os: None,
            features: None,
        }];

        assert!(!parse_rules(&rules, "8", &QuickPlayType::None, false,));
        assert!(parse_rules(&[], "8", &QuickPlayType::None, false,));
    }

    fn loader_version(id: &str, stable: bool) -> LoaderVersion {
        LoaderVersion {
            id: id.to_string(),
            url: format!("https://example.invalid/{id}"),
            stable,
            profile_source: Default::default(),
            fallback_url: None,
        }
    }

    #[test]
    fn liteloader_runtime_detection_uses_the_maven_coordinate() {
        assert!(is_liteloader_library(
            "com.mumfrey:liteloader:1.12.2-SNAPSHOT"
        ));
        assert!(!is_liteloader_library("example:liteloader:1.12.2-SNAPSHOT"));
        assert!(!is_liteloader_library("com.mumfrey:liteloader"));
    }

    #[test]
    fn repair_exact_loader_selection_never_substitutes_or_persists_fallback() {
        let versions = vec![
            loader_version("47.4.22", true),
            loader_version("47.2.0", false),
        ];

        assert_eq!(
            select_loader_version(&versions, Some("47.2.0"))
                .map(|version| version.id.as_str()),
            Some("47.2.0")
        );
        assert!(select_loader_version(&versions, Some("47.1.0")).is_none());
        assert_eq!(
            select_loader_version(&versions, None)
                .map(|version| version.id.as_str()),
            Some("47.4.22")
        );
        assert_eq!(explicit_loader_version(Some("47.1.0")), Some("47.1.0"));
        assert_eq!(explicit_loader_version(None), None);
        assert_eq!(explicit_loader_version(Some("latest")), None);
        assert_eq!(explicit_loader_version(Some("stable")), None);
        assert_eq!(
            missing_loader_version_fallback(
                ModLoader::Forge,
                "1.20.1",
                Some("47.1.0")
            ),
            Err("Loader version 47.1.0 is not available for forge 1.20.1"
                .to_string())
        );
        assert_eq!(
            missing_loader_version_fallback(ModLoader::Forge, "1.20.1", None),
            Ok("stable")
        );
    }

    #[test]
    fn installed_instances_use_persisted_exact_loader_offline() {
        for loader in [
            ModLoader::Fabric,
            ModLoader::Forge,
            ModLoader::NeoForge,
            ModLoader::Quilt,
        ] {
            let version = installed_offline_loader_version(
                loader,
                Some("persisted-exact-version"),
            )
            .expect("installed exact loader should not require metadata");
            assert_eq!(version.id, "persisted-exact-version", "{loader:?}");
            assert!(version.url.is_empty(), "{loader:?}");
        }

        assert!(
            installed_offline_loader_version(ModLoader::Forge, Some("latest"))
                .is_none()
        );
        assert!(
            installed_offline_loader_version(ModLoader::Forge, None).is_none()
        );
    }
}

#[cfg(test)]
mod processor_output_tests {
    use super::*;
    use std::collections::HashMap;

    fn processor(
        path: impl Into<String>,
        hash: impl Into<String>,
    ) -> d::modded::Processor {
        d::modded::Processor {
            jar: "example:processor:1.0".to_string(),
            classpath: Vec::new(),
            args: Vec::new(),
            outputs: Some(HashMap::from([(path.into(), hash.into())])),
            sides: None,
        }
    }

    fn data(
        path: impl Into<String>,
        hash: impl Into<String>,
    ) -> HashMap<String, d::modded::SidedDataEntry> {
        HashMap::from([
            (
                "OUTPUT".to_string(),
                d::modded::SidedDataEntry {
                    client: path.into(),
                    server: String::new(),
                },
            ),
            (
                "HASH".to_string(),
                d::modded::SidedDataEntry {
                    client: hash.into(),
                    server: String::new(),
                },
            ),
        ])
    }

    #[tokio::test]
    async fn valid_processor_output_skips_execution() {
        let directory = tempfile::tempdir().unwrap();
        let libraries = directory.path().join("libraries");
        tokio::fs::create_dir_all(&libraries).await.unwrap();
        let output = libraries.join("output.jar");
        tokio::fs::write(&output, b"valid processor output")
            .await
            .unwrap();
        let hash = sha1_smol::Sha1::from(b"valid processor output").hexdigest();
        let data = data(output.to_string_lossy(), format!("'{hash}'"));

        assert!(
            processor_outputs_are_current(
                &processor("{OUTPUT}", "{HASH}"),
                &libraries,
                &data,
                &[&libraries],
            )
            .await
        );
    }

    #[tokio::test]
    async fn binary_patcher_uses_the_matching_output_sha() {
        let directory = tempfile::tempdir().unwrap();
        let libraries = directory.path().join("libraries");
        tokio::fs::create_dir_all(&libraries).await.unwrap();
        let output = libraries.join("patched.jar");
        tokio::fs::write(&output, b"patched minecraft")
            .await
            .unwrap();
        let hash = sha1_smol::Sha1::from(b"patched minecraft").hexdigest();
        let data = HashMap::from([
            (
                "PATCHED".to_string(),
                d::modded::SidedDataEntry {
                    client: output.to_string_lossy().into_owned(),
                    server: String::new(),
                },
            ),
            (
                "PATCHED_SHA".to_string(),
                d::modded::SidedDataEntry {
                    client: format!("'{hash}'"),
                    server: String::new(),
                },
            ),
        ]);
        let processor = d::modded::Processor {
            jar: "net.minecraftforge:binarypatcher:1.1.1".to_string(),
            classpath: Vec::new(),
            args: vec!["--output".to_string(), "{PATCHED}".to_string()],
            outputs: None,
            sides: None,
        };

        assert!(
            processor_outputs_are_current(
                &processor,
                &libraries,
                &data,
                &[&libraries],
            )
            .await
        );
    }

    #[tokio::test]
    async fn missing_processor_output_runs_execution() {
        let directory = tempfile::tempdir().unwrap();
        let libraries = directory.path().join("libraries");
        tokio::fs::create_dir_all(&libraries).await.unwrap();
        let output = libraries.join("missing.jar");
        let data = data(
            output.to_string_lossy(),
            "0000000000000000000000000000000000000000",
        );

        assert!(
            !processor_outputs_are_current(
                &processor("{OUTPUT}", "{HASH}"),
                &libraries,
                &data,
                &[&libraries],
            )
            .await
        );
    }

    #[tokio::test]
    async fn wrong_processor_output_hash_runs_execution() {
        let directory = tempfile::tempdir().unwrap();
        let libraries = directory.path().join("libraries");
        tokio::fs::create_dir_all(&libraries).await.unwrap();
        let output = libraries.join("output.jar");
        tokio::fs::write(&output, b"wrong hash").await.unwrap();
        let data = data(
            output.to_string_lossy(),
            "0000000000000000000000000000000000000000",
        );

        assert!(
            !processor_outputs_are_current(
                &processor("{OUTPUT}", "{HASH}"),
                &libraries,
                &data,
                &[&libraries],
            )
            .await
        );
    }

    #[tokio::test]
    async fn malformed_or_escaped_processor_outputs_run_execution() {
        let directory = tempfile::tempdir().unwrap();
        let libraries = directory.path().join("libraries");
        tokio::fs::create_dir_all(&libraries).await.unwrap();
        let outside = directory.path().join("outside.jar");
        tokio::fs::write(&outside, b"outside").await.unwrap();
        let hash = sha1_smol::Sha1::from(b"outside").hexdigest();
        let escaped_data = data(outside.to_string_lossy(), &hash);

        assert!(
            !processor_outputs_are_current(
                &processor("{OUTPUT}", "{HASH}"),
                &libraries,
                &escaped_data,
                &[&libraries],
            )
            .await
        );
        let malformed_data = data(outside.to_string_lossy(), "not-a-sha1");
        assert!(
            !processor_outputs_are_current(
                &processor("{OUTPUT}", "{HASH}"),
                &libraries,
                &malformed_data,
                &[directory.path()],
            )
            .await
        );
    }
}

#[cfg(test)]
mod offline_skin_resource_pack_tests {
    use super::*;

    #[test]
    fn enables_offline_skin_pack_when_options_file_is_new() {
        let mut options = String::new();

        update_offline_skin_resource_pack_option(
            &mut options,
            crate::minecraft_skins::OfflineSkinPackOptions {
                enabled_pack_id: Some(
                    crate::minecraft_skins::OFFLINE_SKIN_PACK_MODERN_ID,
                ),
            },
        )
        .unwrap();

        assert_eq!(
            options,
            "resourcePacks:[\"vanilla\",\"file/Axolotl Offline Skin.zip\"]"
        );
    }

    #[test]
    fn adds_modern_offline_skin_pack_without_removing_existing_packs() {
        let mut options =
            "resourcePacks:[\"vanilla\",\"file/user-pack.zip\"]".to_string();

        update_offline_skin_resource_pack_option(
            &mut options,
            crate::minecraft_skins::OfflineSkinPackOptions {
                enabled_pack_id: Some(
                    crate::minecraft_skins::OFFLINE_SKIN_PACK_MODERN_ID,
                ),
            },
        )
        .unwrap();

        assert_eq!(
            options,
            "resourcePacks:[\"vanilla\",\"file/user-pack.zip\",\"file/Axolotl Offline Skin.zip\"]"
        );
    }

    #[test]
    fn removes_both_offline_skin_pack_id_variants() {
        let mut options = "resourcePacks:[\"vanilla\",\"Axolotl Offline Skin.zip\",\"file/Axolotl Offline Skin.zip\"]".to_string();

        update_offline_skin_resource_pack_option(
            &mut options,
            crate::minecraft_skins::OfflineSkinPackOptions {
                enabled_pack_id: None,
            },
        )
        .unwrap();

        assert_eq!(options, "resourcePacks:[\"vanilla\"]");
    }
}
