use crate::State;
use crate::state::CachedLoaderManifest;
use crate::util::fetch::{FetchSemaphore, fetch, fetch_json, fetch_official};
use crate::util::io;
use daedalus::minecraft::Library;
use daedalus::modded::{
    DUMMY_REPLACE_STRING, LoaderProfileSource, LoaderVersion, Manifest,
    PartialVersionInfo, Processor, SidedDataEntry, Version, VersionGroup,
};
use reqwest::Method;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2/versions/";
const QUILT_META_URL: &str = "https://meta.quiltmc.org/v3/versions/";
const FORGE_MAVEN_URL: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/";
const FORGE_PROMOTIONS_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_MAVEN_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/";
const NEOFORGE_LEGACY_MAVEN_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/forge/";
const INSTALLER_URL_KEY: &str = "AXOLOTL_INSTALLER_URL";
const INSTALLER_ENTRY_KEY_PREFIX: &str = "AXOLOTL_INSTALLER_ENTRY_";

#[derive(Deserialize)]
struct MetaGameVersion {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct FabricLoaderVersion {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct QuiltLoaderVersion {
    version: String,
}

#[derive(Deserialize)]
struct FabricLoaderMetadata {
    loader: FabricLoaderVersion,
}

#[derive(Deserialize)]
struct QuiltLoaderMetadata {
    loader: QuiltLoaderVersion,
}

#[derive(Deserialize)]
struct MavenMetadata {
    versioning: MavenVersioning,
}

#[derive(Deserialize)]
struct MavenVersioning {
    #[serde(default)]
    release: String,
    versions: MavenVersions,
}

#[derive(Deserialize)]
struct MavenVersions {
    #[serde(rename = "version", default)]
    values: Vec<String>,
}

#[derive(Deserialize, Default)]
struct ForgePromotions {
    #[serde(default)]
    promos: HashMap<String, String>,
}

#[derive(Deserialize)]
struct InstallerProfile {
    #[serde(default)]
    data: HashMap<String, SidedDataEntry>,
    #[serde(default)]
    processors: Vec<Processor>,
    #[serde(default)]
    libraries: Vec<Library>,
}

struct InstallerArtifact {
    coordinate: String,
    data: Vec<u8>,
}

struct ParsedInstaller {
    profile: PartialVersionInfo,
    artifacts: Vec<InstallerArtifact>,
}

pub(crate) async fn fetch_loader_manifest_official_first(
    loader: &str,
    game_version: Option<&str>,
    fallback_url: &str,
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<CachedLoaderManifest> {
    let official = match (loader, game_version) {
        ("fabric", Some(game_version)) => {
            fetch_fabric_manifest_for_game(game_version, fetch_semaphore, pool)
                .await
        }
        ("quilt", Some(game_version)) => {
            fetch_quilt_manifest_for_game(game_version, fetch_semaphore, pool)
                .await
        }
        ("forge", Some(game_version)) => {
            fetch_forge_manifest(fetch_semaphore, pool)
                .await
                .map(|manifest| scope_manifest(manifest, game_version))
        }
        ("neo", Some(game_version)) => {
            fetch_neoforge_manifest(fetch_semaphore, pool)
                .await
                .map(|manifest| scope_manifest(manifest, game_version))
        }
        ("fabric", None) => fetch_fabric_manifest(fetch_semaphore, pool).await,
        ("forge", None) => fetch_forge_manifest(fetch_semaphore, pool).await,
        ("neo", None) => fetch_neoforge_manifest(fetch_semaphore, pool).await,
        ("quilt", None) => fetch_quilt_manifest(fetch_semaphore, pool).await,
        _ => {
            return fetch_fallback_manifest(
                loader,
                game_version,
                fallback_url,
                fetch_semaphore,
                pool,
            )
            .await;
        }
    };

    let official = official.and_then(|manifest| {
        if let Some(game_version) = game_version {
            validate_scoped_manifest(manifest, game_version)
        } else {
            validate_manifest(manifest)
        }
    });

    match official {
        Ok(manifest) => Ok(CachedLoaderManifest {
            loader: loader.to_string(),
            manifest,
        }),
        Err(error) => {
            tracing::warn!(
                loader,
                fallback_url,
                error = %error,
                "Official loader metadata failed; using launcher-meta fallback"
            );
            fetch_fallback_manifest(
                loader,
                game_version,
                fallback_url,
                fetch_semaphore,
                pool,
            )
            .await
        }
    }
}

async fn fetch_fallback_manifest(
    loader: &str,
    game_version: Option<&str>,
    fallback_url: &str,
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<CachedLoaderManifest> {
    let manifest: Manifest = fetch_json(
        Method::GET,
        fallback_url,
        None,
        None,
        None,
        fetch_semaphore,
        pool,
    )
    .await?;
    let manifest = if let Some(game_version) = game_version {
        scope_manifest(manifest, game_version)
    } else {
        manifest
    };
    Ok(CachedLoaderManifest {
        loader: loader.to_string(),
        manifest,
    })
}

fn validate_scoped_manifest(
    manifest: Manifest,
    game_version: &str,
) -> crate::Result<Manifest> {
    if manifest.game_versions.len() != 1
        || manifest.game_versions[0].id != game_version
    {
        return Err(crate::ErrorKind::OtherError(format!(
            "Loader metadata response was not scoped to Minecraft {game_version}"
        ))
        .as_error());
    }
    Ok(manifest)
}

fn validate_manifest(manifest: Manifest) -> crate::Result<Manifest> {
    let loader_count = manifest
        .game_versions
        .iter()
        .map(|version| version.loaders.len())
        .chain(
            manifest
                .version_groups
                .iter()
                .map(|group| group.loaders.len()),
        )
        .sum::<usize>();
    if manifest.game_versions.is_empty() || loader_count == 0 {
        return Err(crate::ErrorKind::OtherError(
            "Official loader metadata returned no installable versions"
                .to_string(),
        )
        .as_error());
    }
    Ok(manifest)
}

async fn fetch_fabric_manifest(
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<Manifest> {
    let game_url = format!("{FABRIC_META_URL}game");
    let loader_url = format!("{FABRIC_META_URL}loader");
    let (games, loaders) = tokio::try_join!(
        fetch_json::<Vec<MetaGameVersion>>(
            Method::GET,
            &game_url,
            None,
            None,
            None,
            fetch_semaphore,
            pool,
        ),
        fetch_json::<Vec<FabricLoaderVersion>>(
            Method::GET,
            &loader_url,
            None,
            None,
            None,
            fetch_semaphore,
            pool,
        ),
    )?;
    Ok(meta_api_manifest(
        games,
        loaders
            .into_iter()
            .map(|loader| (loader.version, loader.stable))
            .collect(),
        "fabric",
        FABRIC_META_URL,
        0,
    ))
}

async fn fetch_fabric_manifest_for_game(
    game_version: &str,
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<Manifest> {
    let url = meta_loader_versions_url(FABRIC_META_URL, game_version);
    let loaders = fetch_json::<Vec<FabricLoaderMetadata>>(
        Method::GET,
        &url,
        None,
        None,
        None,
        fetch_semaphore,
        pool,
    )
    .await?
    .into_iter()
    .map(|metadata| (metadata.loader.version, metadata.loader.stable))
    .collect();
    Ok(meta_api_manifest_for_game(
        game_version,
        loaders,
        "fabric",
        FABRIC_META_URL,
        0,
    ))
}

async fn fetch_quilt_manifest(
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<Manifest> {
    let game_url = format!("{QUILT_META_URL}game");
    let loader_url = format!("{QUILT_META_URL}loader");
    let (games, loaders) = tokio::try_join!(
        fetch_json::<Vec<MetaGameVersion>>(
            Method::GET,
            &game_url,
            None,
            None,
            None,
            fetch_semaphore,
            pool,
        ),
        fetch_json::<Vec<QuiltLoaderVersion>>(
            Method::GET,
            &loader_url,
            None,
            None,
            None,
            fetch_semaphore,
            pool,
        ),
    )?;
    Ok(meta_api_manifest(
        games,
        loaders
            .into_iter()
            .map(|loader| {
                let stable = is_stable_version(&loader.version);
                (loader.version, stable)
            })
            .collect(),
        "quilt",
        QUILT_META_URL,
        1,
    ))
}

async fn fetch_quilt_manifest_for_game(
    game_version: &str,
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<Manifest> {
    let url = meta_loader_versions_url(QUILT_META_URL, game_version);
    let loaders = fetch_json::<Vec<QuiltLoaderMetadata>>(
        Method::GET,
        &url,
        None,
        None,
        None,
        fetch_semaphore,
        pool,
    )
    .await?
    .into_iter()
    .map(|metadata| {
        let version = metadata.loader.version;
        let stable = is_stable_version(&version);
        (version, stable)
    })
    .collect();
    Ok(meta_api_manifest_for_game(
        game_version,
        loaders,
        "quilt",
        QUILT_META_URL,
        1,
    ))
}

fn meta_loader_versions_url(base_url: &str, game_version: &str) -> String {
    format!("{base_url}loader/{game_version}")
}

fn meta_api_manifest_for_game(
    game_version: &str,
    mut loaders: Vec<(String, bool)>,
    loader: &str,
    official_base_url: &str,
    fallback_format_version: usize,
) -> Manifest {
    loaders.sort_by(|left, right| compare_versions(&right.0, &left.0));
    Manifest {
        game_versions: vec![Version {
            id: game_version.to_string(),
            stable: true,
            version_group: None,
            loaders: loaders
                .into_iter()
                .map(|(version, stable)| LoaderVersion {
                    id: version.clone(),
                    url: format!(
                        "{official_base_url}loader/{game_version}/{version}/profile/json"
                    ),
                    stable,
                    profile_source: LoaderProfileSource::Json,
                    fallback_url: Some(fallback_profile_url(
                        loader,
                        fallback_format_version,
                        &version,
                    )),
                })
                .collect(),
        }],
        version_groups: Vec::new(),
    }
}

fn meta_api_manifest(
    games: Vec<MetaGameVersion>,
    mut loaders: Vec<(String, bool)>,
    loader: &str,
    official_base_url: &str,
    fallback_format_version: usize,
) -> Manifest {
    loaders.sort_by(|left, right| compare_versions(&right.0, &left.0));
    let group_id = format!("{loader}-official");
    let loader_versions = loaders
        .into_iter()
        .map(|(version, stable)| LoaderVersion {
            id: version.clone(),
            url: format!(
                "{official_base_url}loader/{DUMMY_REPLACE_STRING}/{version}/profile/json"
            ),
            stable,
            profile_source: LoaderProfileSource::Json,
            fallback_url: Some(format!(
                "{}{}{}{}{}",
                env!("MODRINTH_LAUNCHER_META_URL"),
                loader,
                "/v",
                fallback_format_version,
                format!("/versions/{version}.json"),
            )),
        })
        .collect();
    Manifest {
        game_versions: games
            .into_iter()
            .map(|game| Version {
                id: game.version,
                stable: game.stable,
                version_group: Some(group_id.clone()),
                loaders: Vec::new(),
            })
            .collect(),
        version_groups: vec![VersionGroup {
            id: group_id,
            loaders: loader_versions,
        }],
    }
}

async fn fetch_forge_manifest(
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<Manifest> {
    let metadata_url = format!("{FORGE_MAVEN_URL}maven-metadata.xml");
    let metadata =
        fetch_forge_maven_metadata(&metadata_url, fetch_semaphore, pool)
            .await?;
    let promotions = fetch_json::<ForgePromotions>(
        Method::GET,
        FORGE_PROMOTIONS_URL,
        None,
        None,
        None,
        fetch_semaphore,
        pool,
    )
    .await
    .unwrap_or_default();

    Ok(forge_manifest(metadata, promotions))
}

fn forge_manifest(
    metadata: MavenMetadata,
    promotions: ForgePromotions,
) -> Manifest {
    let mut grouped: HashMap<String, Vec<LoaderVersion>> = HashMap::new();
    for full_version in metadata.versioning.versions.values {
        let Some((game_version, loader_version)) = full_version.split_once('-')
        else {
            continue;
        };
        let recommended = promotions
            .promos
            .get(&format!("{game_version}-recommended"));
        let stable =
            recommended.is_some_and(|promoted| promoted == loader_version);
        add_forge_loader_version(
            &mut grouped,
            game_version,
            loader_version,
            stable,
        );
    }

    let mut manifest = grouped_manifest(grouped);
    for game_version in &mut manifest.game_versions {
        let Some(latest) = promotions
            .promos
            .get(&format!("{}-latest", game_version.id))
        else {
            continue;
        };
        let Some(index) = game_version
            .loaders
            .iter()
            .position(|loader| loader.id == *latest)
        else {
            continue;
        };
        if index != 0 {
            let latest = game_version.loaders.remove(index);
            game_version.loaders.insert(0, latest);
        }
    }
    manifest
}

fn add_forge_loader_version(
    grouped: &mut HashMap<String, Vec<LoaderVersion>>,
    game_version: &str,
    loader_version: &str,
    stable: bool,
) {
    let loaders = grouped.entry(game_version.to_string()).or_default();
    if let Some(existing) = loaders
        .iter_mut()
        .find(|version| version.id == loader_version)
    {
        existing.stable |= stable;
        return;
    }

    let full_version = format!("{game_version}-{loader_version}");
    loaders.push(LoaderVersion {
        id: loader_version.to_string(),
        url: format!(
            "{FORGE_MAVEN_URL}{full_version}/forge-{full_version}-installer.jar"
        ),
        stable,
        profile_source: LoaderProfileSource::Installer,
        fallback_url: Some(fallback_profile_url("forge", 0, loader_version)),
    });
}

async fn fetch_neoforge_manifest(
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<Manifest> {
    let current_url = format!("{NEOFORGE_MAVEN_URL}maven-metadata.xml");
    let legacy_url = format!("{NEOFORGE_LEGACY_MAVEN_URL}maven-metadata.xml");
    let (current, legacy) = tokio::try_join!(
        fetch_maven_metadata(&current_url, fetch_semaphore, pool),
        fetch_maven_metadata(&legacy_url, fetch_semaphore, pool),
    )?;
    Ok(neoforge_manifest(current, legacy))
}

fn neoforge_manifest(
    current: MavenMetadata,
    legacy: MavenMetadata,
) -> Manifest {
    let current_release = current.versioning.release;
    let legacy_release = legacy.versioning.release;
    let mut grouped: HashMap<String, Vec<LoaderVersion>> = HashMap::new();

    for version in current.versioning.versions.values {
        let Some(game_version) = neoforge_game_version(&version) else {
            continue;
        };
        grouped
            .entry(game_version)
            .or_default()
            .push(LoaderVersion {
                id: version.clone(),
                url: format!(
                    "{NEOFORGE_MAVEN_URL}{version}/neoforge-{version}-installer.jar"
                ),
                stable: version == current_release
                    || is_stable_version(&version),
                profile_source: LoaderProfileSource::Installer,
                fallback_url: Some(fallback_profile_url(
                    "neo", 0, &version,
                )),
            });
    }

    for full_version in legacy.versioning.versions.values {
        let Some((game_version, loader_version)) = full_version.split_once('-')
        else {
            continue;
        };
        grouped
            .entry(game_version.to_string())
            .or_default()
            .push(LoaderVersion {
                id: loader_version.to_string(),
                url: format!(
                    "{NEOFORGE_LEGACY_MAVEN_URL}{full_version}/forge-{full_version}-installer.jar"
                ),
                stable: full_version == legacy_release
                    || is_stable_version(loader_version),
                profile_source: LoaderProfileSource::Installer,
                fallback_url: Some(fallback_profile_url(
                    "neo",
                    0,
                    loader_version,
                )),
            });
    }

    grouped_manifest(grouped)
}

fn grouped_manifest(
    mut grouped: HashMap<String, Vec<LoaderVersion>>,
) -> Manifest {
    let mut game_versions = grouped
        .drain()
        .map(|(game_version, mut loaders)| {
            loaders
                .sort_by(|left, right| compare_versions(&right.id, &left.id));
            Version {
                id: game_version,
                stable: true,
                version_group: None,
                loaders,
            }
        })
        .collect::<Vec<_>>();
    game_versions.sort_by(|left, right| compare_versions(&right.id, &left.id));
    Manifest {
        game_versions,
        version_groups: Vec::new(),
    }
}

fn scope_manifest(manifest: Manifest, game_version: &str) -> Manifest {
    let has_explicit_game_version = manifest
        .game_versions
        .iter()
        .any(|version| version.id == game_version);
    let scoped = manifest.game_versions.iter().find(|version| {
        version.id.replace(DUMMY_REPLACE_STRING, game_version) == game_version
    });

    let (stable, loaders) = scoped.map_or((true, Vec::new()), |version| {
        let loaders = if version.id == DUMMY_REPLACE_STRING
            && !has_explicit_game_version
        {
            Vec::new()
        } else if let Some(group_id) = &version.version_group {
            manifest
                .version_groups
                .iter()
                .find(|group| group.id == *group_id)
                .map(|group| group.loaders.clone())
                .unwrap_or_default()
        } else {
            version.loaders.clone()
        };
        (version.stable, loaders)
    });

    Manifest {
        game_versions: vec![Version {
            id: game_version.to_string(),
            stable,
            version_group: None,
            loaders,
        }],
        version_groups: Vec::new(),
    }
}

async fn fetch_maven_metadata(
    url: &str,
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<MavenMetadata> {
    let bytes = fetch(url, None, None, None, fetch_semaphore, pool).await?;
    let xml = std::str::from_utf8(&bytes).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Maven metadata at {url} was not UTF-8: {error}"
        ))
        .as_error()
    })?;
    quick_xml::de::from_str(xml).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Failed to parse Maven metadata at {url}: {error}"
        ))
        .as_error()
    })
}

async fn fetch_forge_maven_metadata(
    url: &str,
    fetch_semaphore: &FetchSemaphore,
    pool: &SqlitePool,
) -> crate::Result<MavenMetadata> {
    let bytes =
        fetch_official(url, None, None, None, fetch_semaphore, pool).await?;
    let xml = std::str::from_utf8(&bytes).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Forge Maven metadata at {url} was not UTF-8: {error}"
        ))
        .as_error()
    })?;
    quick_xml::de::from_str(xml).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Failed to parse Forge Maven metadata at {url}: {error}"
        ))
        .as_error()
    })
}

fn fallback_profile_url(
    loader: &str,
    format_version: usize,
    version: &str,
) -> String {
    format!(
        "{}{loader}/v{format_version}/versions/{version}.json",
        env!("MODRINTH_LAUNCHER_META_URL")
    )
}

fn neoforge_game_version(version: &str) -> Option<String> {
    if let Some(snapshot) = version.strip_prefix("0.") {
        let snapshot = snapshot.rsplit_once('.')?.0;
        return Some(format!("1.0.{snapshot}"));
    }

    let components = version.split('.').collect::<Vec<_>>();
    let major = components.first()?.parse::<u32>().ok()?;
    let minor = components.get(1)?.parse::<u32>().ok()?;
    if major >= 26 {
        let patch = components.get(2)?.parse::<u32>().ok()?;
        if patch == 0 {
            Some(format!("{major}.{minor}"))
        } else {
            Some(format!("{major}.{minor}.{patch}"))
        }
    } else if minor == 0 {
        Some(format!("1.{major}"))
    } else {
        Some(format!("1.{major}.{minor}"))
    }
}

fn is_stable_version(version: &str) -> bool {
    let version = version.to_ascii_lowercase();
    !["alpha", "beta", "rc", "snapshot", "pre"]
        .iter()
        .any(|marker| version.contains(marker))
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let mut left = left.as_bytes().iter().peekable();
    let mut right = right.as_bytes().iter().peekable();
    loop {
        match (left.peek(), right.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Greater,
            (Some(_), None) => return Ordering::Less,
            (Some(left_byte), Some(right_byte)) => {
                let left_is_digit = left_byte.is_ascii_digit();
                let right_is_digit = right_byte.is_ascii_digit();
                if left_is_digit != right_is_digit {
                    return left_byte.cmp(right_byte);
                }

                let mut left_part = Vec::new();
                let mut right_part = Vec::new();
                while left
                    .peek()
                    .is_some_and(|byte| byte.is_ascii_digit() == left_is_digit)
                {
                    left_part.push(**left.peek().unwrap());
                    left.next();
                }
                while right
                    .peek()
                    .is_some_and(|byte| byte.is_ascii_digit() == right_is_digit)
                {
                    right_part.push(**right.peek().unwrap());
                    right.next();
                }

                let ordering = if left_is_digit {
                    left_part
                        .len()
                        .cmp(&right_part.len())
                        .then_with(|| left_part.cmp(&right_part))
                } else {
                    left_part.cmp(&right_part)
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

pub(crate) async fn resolve_loader_profile(
    state: &State,
    game_version: &str,
    loader_version: &LoaderVersion,
) -> crate::Result<PartialVersionInfo> {
    let primary_url = loader_version
        .url
        .replace(DUMMY_REPLACE_STRING, game_version);
    let primary = match loader_version.profile_source {
        LoaderProfileSource::Json => {
            fetch_json(
                Method::GET,
                &primary_url,
                None,
                None,
                None,
                &state.api_semaphore,
                &state.pool,
            )
            .await
        }
        LoaderProfileSource::Installer => {
            resolve_installer_profile(state, &primary_url).await
        }
    };

    match primary {
        Ok(profile) => Ok(profile),
        Err(primary_error) => {
            let Some(fallback_url) = &loader_version.fallback_url else {
                return Err(primary_error);
            };
            let fallback_url =
                fallback_url.replace(DUMMY_REPLACE_STRING, game_version);
            tracing::warn!(
                loader_version = loader_version.id,
                primary_url,
                fallback_url,
                error = %primary_error,
                "Official loader profile failed; using launcher-meta fallback"
            );
            fetch_json(
                Method::GET,
                &fallback_url,
                None,
                None,
                None,
                &state.api_semaphore,
                &state.pool,
            )
            .await
        }
    }
}

async fn resolve_installer_profile(
    state: &State,
    installer_url: &str,
) -> crate::Result<PartialVersionInfo> {
    let bytes = fetch(
        installer_url,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    let installer_url = installer_url.to_string();
    let parsed = tokio::task::spawn_blocking(move || {
        parse_installer(bytes.to_vec(), &installer_url)
    })
    .await??;
    persist_installer_artifacts(state, &parsed.artifacts).await?;
    Ok(parsed.profile)
}

pub(crate) async fn ensure_installer_artifacts(
    state: &State,
    version: &daedalus::minecraft::VersionInfo,
) -> crate::Result<()> {
    let Some(data) = &version.data else {
        return Ok(());
    };
    let Some(installer_url) = data.get(INSTALLER_URL_KEY) else {
        return Ok(());
    };
    let entries = data
        .iter()
        .filter(|(key, _)| key.starts_with(INSTALLER_ENTRY_KEY_PREFIX))
        .filter_map(|(_, entry)| entry.client.split_once('|'))
        .map(|(coordinate, archive_path)| {
            (coordinate.to_string(), archive_path.to_string())
        })
        .collect::<Vec<_>>();
    let mut missing = Vec::new();
    for (coordinate, archive_path) in entries {
        let relative = daedalus::get_path_from_artifact(&coordinate)?;
        if !state.directories.libraries_dir().join(relative).exists() {
            missing.push((coordinate, archive_path));
        }
    }
    if missing.is_empty() {
        return Ok(());
    }

    let bytes = fetch(
        &installer_url.client,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    let artifacts = tokio::task::spawn_blocking(move || {
        extract_installer_artifacts(bytes.to_vec(), &missing)
    })
    .await??;
    persist_installer_artifacts(state, &artifacts).await
}

fn parse_installer(
    bytes: Vec<u8>,
    installer_url: &str,
) -> crate::Result<ParsedInstaller> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Failed to open loader installer: {error}"
            ))
            .as_error()
        })?;
    let version_json = read_zip_entry(&mut archive, "version.json")?;
    let profile_json = read_zip_entry(&mut archive, "install_profile.json")?;
    let mut partial: PartialVersionInfo =
        serde_json::from_slice(&version_json)?;
    let mut installer: InstallerProfile =
        serde_json::from_slice(&profile_json)?;

    for library in &mut installer.libraries {
        library.include_in_classpath = false;
    }
    for library in &mut partial.libraries {
        if library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.as_ref())
            .is_some_and(|artifact| artifact.url.is_empty())
        {
            library.downloadable = false;
        }
    }

    let existing = partial
        .libraries
        .iter()
        .map(|library| library.name.clone())
        .collect::<HashSet<_>>();
    partial.libraries.extend(
        installer
            .libraries
            .into_iter()
            .filter(|library| !existing.contains(&library.name)),
    );
    partial.processors = Some(installer.processors);

    let requests = installer
        .data
        .iter()
        .flat_map(|(key, entry)| {
            [("client", &entry.client), ("server", &entry.server)]
                .into_iter()
                .filter_map(move |(side, value)| {
                    value
                        .strip_prefix('/')
                        .map(|path| (key.clone(), side, path.to_string()))
                })
        })
        .collect::<Vec<_>>();
    let mut artifacts = Vec::new();
    for (index, (key, side, archive_path)) in requests.into_iter().enumerate() {
        let artifact_data = read_zip_entry(&mut archive, &archive_path)?;
        let extension = std::path::Path::new(&archive_path)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("bin");
        let coordinate = format!(
            "com.axolotl.loader-installer:embedded:{}:{}-{}@{}",
            sanitize_coordinate_part(&partial.id),
            sanitize_coordinate_part(&key),
            side,
            sanitize_coordinate_part(extension),
        );
        let entry = installer.data.get_mut(&key).unwrap();
        let replacement = format!("[{coordinate}]");
        if side == "client" {
            entry.client = replacement;
        } else {
            entry.server = replacement;
        }
        installer.data.insert(
            format!("{INSTALLER_ENTRY_KEY_PREFIX}{index}"),
            SidedDataEntry {
                client: format!("{coordinate}|{archive_path}"),
                server: format!("{coordinate}|{archive_path}"),
            },
        );
        partial.libraries.push(local_library(coordinate.clone()));
        artifacts.push(InstallerArtifact {
            coordinate,
            data: artifact_data,
        });
    }
    installer.data.insert(
        INSTALLER_URL_KEY.to_string(),
        SidedDataEntry {
            client: installer_url.to_string(),
            server: installer_url.to_string(),
        },
    );
    partial.data = Some(installer.data);

    Ok(ParsedInstaller {
        profile: partial,
        artifacts,
    })
}

fn extract_installer_artifacts(
    bytes: Vec<u8>,
    entries: &[(String, String)],
) -> crate::Result<Vec<InstallerArtifact>> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Failed to open cached loader installer: {error}"
            ))
            .as_error()
        })?;
    entries
        .iter()
        .map(|(coordinate, archive_path)| {
            Ok(InstallerArtifact {
                coordinate: coordinate.clone(),
                data: read_zip_entry(&mut archive, archive_path)?,
            })
        })
        .collect()
}

fn read_zip_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    path: &str,
) -> crate::Result<Vec<u8>> {
    let mut entry = archive.by_name(path).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Loader installer is missing {path}: {error}"
        ))
        .as_error()
    })?;
    let mut data = Vec::new();
    entry.read_to_end(&mut data)?;
    Ok(data)
}

async fn persist_installer_artifacts(
    state: &State,
    artifacts: &[InstallerArtifact],
) -> crate::Result<()> {
    for artifact in artifacts {
        let relative = daedalus::get_path_from_artifact(&artifact.coordinate)?;
        let path = state.directories.libraries_dir().join(relative);
        if let Some(parent) = path.parent() {
            io::create_dir_all(parent).await?;
        }
        io::write(path, &artifact.data).await?;
    }
    Ok(())
}

fn local_library(name: String) -> Library {
    Library {
        downloads: None,
        extract: None,
        name,
        url: None,
        natives: None,
        rules: None,
        checksums: None,
        include_in_classpath: false,
        downloadable: false,
    }
}

fn sanitize_coordinate_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || matches!(character, '.' | '-' | '_')
            {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn maven_metadata(release: &str, versions: &[&str]) -> MavenMetadata {
        MavenMetadata {
            versioning: MavenVersioning {
                release: release.to_string(),
                versions: MavenVersions {
                    values: versions
                        .iter()
                        .map(|version| version.to_string())
                        .collect(),
                },
            },
        }
    }

    fn forge_loaders<'a>(
        manifest: &'a Manifest,
        game_version: &str,
    ) -> &'a [LoaderVersion] {
        &manifest
            .game_versions
            .iter()
            .find(|version| version.id == game_version)
            .unwrap_or_else(|| {
                panic!("missing Forge support for {game_version}")
            })
            .loaders
    }

    #[test]
    fn forge_metadata_groups_versions_and_uses_installer_profiles() {
        let manifest = forge_manifest(
            maven_metadata(
                "1.21.5-55.1.13",
                &["1.21.5-55.1.9", "1.21.5-55.1.13"],
            ),
            ForgePromotions {
                promos: HashMap::from([
                    ("1.21.5-latest".to_string(), "55.1.13".to_string()),
                    ("1.21.5-recommended".to_string(), "55.1.9".to_string()),
                ]),
            },
        );
        let loaders = &manifest.game_versions[0].loaders;
        assert_eq!(loaders[0].id, "55.1.13");
        assert_eq!(loaders[0].profile_source, LoaderProfileSource::Installer);
        assert!(!loaders[0].stable);
        assert!(loaders[1].stable);
        assert!(
            loaders[0].url.ends_with(
                "/1.21.5-55.1.13/forge-1.21.5-55.1.13-installer.jar"
            )
        );
    }

    #[test]
    fn forge_maven_metadata_parses_modern_version_coordinates() {
        let metadata: MavenMetadata = quick_xml::de::from_str(
            r#"<metadata>
                <versioning>
                    <release>26.2-65.1.0</release>
                    <versions>
                        <version>1.18.2-40.3.12</version>
                        <version>1.19-41.1.0</version>
                        <version>1.20.1-47.4.22</version>
                        <version>1.21.1-52.1.16</version>
                        <version>26.1-62.0.9</version>
                        <version>26.2-65.1.0</version>
                    </versions>
                </versioning>
            </metadata>"#,
        )
        .unwrap();
        let manifest = forge_manifest(metadata, ForgePromotions::default());

        for (game_version, loader_prefix) in [
            ("1.18.2", "40."),
            ("1.19", "41."),
            ("1.20.1", "47."),
            ("1.21.1", "52."),
            ("26.1", "62."),
            ("26.2", "65."),
        ] {
            assert!(
                forge_loaders(&manifest, game_version)
                    .iter()
                    .any(|loader| loader.id.starts_with(loader_prefix)),
                "missing Forge {loader_prefix} for Minecraft {game_version}"
            );
        }
    }

    #[test]
    fn forge_preserves_complete_maven_catalog_and_maps_promotions() {
        let metadata: MavenMetadata = quick_xml::de::from_str(
            r#"<metadata>
                <versioning>
                    <release>26.2-65.1.1</release>
                    <versions>
                        <version>26.2-65.1.1</version>
                        <version>26.2-65.1.0</version>
                        <version>26.2-65.0.9</version>
                        <version>26.2-65.0.8</version>
                        <version>26.2-65.0.1</version>
                        <version>26.2-65.0.0</version>
                        <version>26.1-62.0.9</version>
                    </versions>
                </versioning>
            </metadata>"#,
        )
        .unwrap();
        let manifest = forge_manifest(
            metadata,
            ForgePromotions {
                promos: HashMap::from([
                    ("26.2-latest".to_string(), "65.1.1".to_string()),
                    ("26.2-recommended".to_string(), "65.1.0".to_string()),
                ]),
            },
        );
        let loaders = forge_loaders(&manifest, "26.2");

        assert_eq!(
            loaders
                .iter()
                .map(|loader| loader.id.as_str())
                .collect::<Vec<_>>(),
            ["65.1.1", "65.1.0", "65.0.9", "65.0.8", "65.0.1", "65.0.0"]
        );
        assert!(!loaders[0].stable);
        assert!(loaders[1].stable);
        assert!(loaders[2..].iter().all(|loader| !loader.stable));
        assert!(
            forge_loaders(&manifest, "26.1")
                .iter()
                .all(|loader| !loader.id.starts_with("65."))
        );
    }

    #[test]
    fn forge_promotions_do_not_create_catalog_entries() {
        let manifest = forge_manifest(
            maven_metadata("26.2-65.1.0", &["26.2-65.1.0"]),
            ForgePromotions {
                promos: HashMap::from([(
                    "26.2-latest".to_string(),
                    "65.1.1".to_string(),
                )]),
            },
        );

        assert_eq!(forge_loaders(&manifest, "26.2")[0].id, "65.1.0");
    }

    #[test]
    fn forge_scoped_metadata_keeps_empty_and_supported_versions_isolated() {
        let full = forge_manifest(
            maven_metadata(
                "1.20.1-47.4.22",
                &["1.20.1-47.4.21", "1.20.1-47.4.22", "1.21.5-55.1.13"],
            ),
            ForgePromotions::default(),
        );

        let forge_262 = scope_manifest(full.clone(), "26.2");
        let forge_1201 = scope_manifest(full.clone(), "1.20.1");
        let forge_262_again = scope_manifest(full, "26.2");

        assert!(forge_262.game_versions[0].loaders.is_empty());
        assert_eq!(
            forge_1201.game_versions[0]
                .loaders
                .iter()
                .map(|loader| loader.id.as_str())
                .collect::<Vec<_>>(),
            ["47.4.22", "47.4.21"]
        );
        assert!(forge_262_again.game_versions[0].loaders.is_empty());
        assert!(validate_scoped_manifest(forge_262, "26.2").is_ok());
    }

    #[test]
    fn forge_scoped_metadata_preserves_complete_catalog_across_switches() {
        let full = forge_manifest(
            maven_metadata(
                "26.2-65.1.1",
                &[
                    "26.2-65.1.1",
                    "26.2-65.1.0",
                    "26.2-65.0.9",
                    "1.20.1-47.4.22",
                    "1.20.1-47.4.21",
                ],
            ),
            ForgePromotions::default(),
        );

        let forge_262 = scope_manifest(full.clone(), "26.2");
        let forge_1201 = scope_manifest(full.clone(), "1.20.1");
        let forge_262_again = scope_manifest(full, "26.2");
        let ids = |manifest: &Manifest| {
            manifest.game_versions[0]
                .loaders
                .iter()
                .map(|loader| loader.id.clone())
                .collect::<Vec<_>>()
        };

        assert_eq!(ids(&forge_262), ["65.1.1", "65.1.0", "65.0.9"]);
        assert_eq!(ids(&forge_1201), ["47.4.22", "47.4.21"]);
        assert_eq!(ids(&forge_262_again), ids(&forge_262));
    }

    #[test]
    fn meta_api_manifest_uses_official_profiles_and_fallbacks() {
        let manifest = meta_api_manifest(
            vec![MetaGameVersion {
                version: "1.21.1".to_string(),
                stable: true,
            }],
            vec![("0.16.9".to_string(), false), ("0.16.14".to_string(), true)],
            "fabric",
            FABRIC_META_URL,
            0,
        );
        let version = &manifest.game_versions[0];
        let loaders = &manifest.version_groups[0].loaders;
        assert_eq!(version.version_group.as_deref(), Some("fabric-official"));
        assert_eq!(loaders[0].id, "0.16.14");
        assert!(
            loaders[0]
                .url
                .contains("${modrinth.gameVersion}/0.16.14/profile/json")
        );
        assert!(
            loaders[0]
                .fallback_url
                .as_ref()
                .unwrap()
                .ends_with("fabric/v0/versions/0.16.14.json")
        );
    }

    #[test]
    fn scoped_meta_manifest_binds_profile_and_versions_to_requested_game() {
        assert_eq!(
            meta_loader_versions_url(FABRIC_META_URL, "1.20.1"),
            "https://meta.fabricmc.net/v2/versions/loader/1.20.1"
        );
        assert_eq!(
            meta_loader_versions_url(QUILT_META_URL, "26.2"),
            "https://meta.quiltmc.org/v3/versions/loader/26.2"
        );
        let manifest = meta_api_manifest_for_game(
            "1.20.1",
            vec![("0.16.9".to_string(), false), ("0.16.14".to_string(), true)],
            "fabric",
            FABRIC_META_URL,
            0,
        );

        let game = &manifest.game_versions[0];
        assert_eq!(game.id, "1.20.1");
        assert_eq!(game.loaders[0].id, "0.16.14");
        assert!(
            game.loaders[0]
                .url
                .contains("loader/1.20.1/0.16.14/profile/json")
        );
        assert!(!game.loaders[0].url.contains(DUMMY_REPLACE_STRING));
    }

    #[test]
    fn neoforge_scoped_metadata_does_not_mix_build_series() {
        let full = neoforge_manifest(
            maven_metadata("21.2.10", &["21.1.200", "21.1.201", "21.2.10"]),
            maven_metadata("", &[]),
        );

        let mc_1211 = scope_manifest(full.clone(), "1.21.1");
        let mc_1212 = scope_manifest(full.clone(), "1.21.2");
        let mc_1211_again = scope_manifest(full, "1.21.1");

        assert!(
            mc_1211.game_versions[0]
                .loaders
                .iter()
                .all(|loader| loader.id.starts_with("21.1."))
        );
        assert!(
            mc_1212.game_versions[0]
                .loaders
                .iter()
                .all(|loader| loader.id.starts_with("21.2."))
        );
        assert_eq!(
            mc_1211.game_versions[0]
                .loaders
                .iter()
                .map(|loader| loader.id.as_str())
                .collect::<Vec<_>>(),
            mc_1211_again.game_versions[0]
                .loaders
                .iter()
                .map(|loader| loader.id.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn neoforge_versions_map_to_minecraft_versions() {
        assert_eq!(neoforge_game_version("20.2.93"), Some("1.20.2".into()));
        assert_eq!(neoforge_game_version("21.0.167"), Some("1.21".into()));
        assert_eq!(neoforge_game_version("21.11.45"), Some("1.21.11".into()));
        assert_eq!(
            neoforge_game_version("26.1.0.19-beta"),
            Some("26.1".into())
        );
        assert_eq!(neoforge_game_version("26.1.2.95"), Some("26.1.2".into()));
        assert_eq!(
            neoforge_game_version("0.25w14craftmine.5-beta"),
            Some("1.0.25w14craftmine".into())
        );
    }

    #[test]
    fn installer_parser_merges_profile_and_embedded_artifacts() {
        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        archive.start_file("version.json", options).unwrap();
        archive
            .write_all(
                br#"{
                    "id":"1.21.5-forge-55.1.13",
                    "inheritsFrom":"1.21.5",
                    "releaseTime":"2026-08-15T20:37:45+00:00",
                    "time":"2026-08-15T20:37:45+00:00",
                    "mainClass":"example.Main",
                    "libraries":[{
                        "name":"example:generated:1.0",
                        "downloads":{"artifact":{"path":"example/generated/1.0/generated-1.0.jar","sha1":"abc","size":1,"url":""}}
                    }],
                    "type":"release"
                }"#,
            )
            .unwrap();
        archive.start_file("install_profile.json", options).unwrap();
        archive
            .write_all(
                br#"{
                    "data":{"BINPATCH":{"client":"/data/client.lzma","server":"/data/server.lzma"}},
                    "processors":[{"jar":"example:processor:1.0","classpath":[],"args":["{BINPATCH}"]}],
                    "libraries":[{"name":"example:processor:1.0"}]
                }"#,
            )
            .unwrap();
        archive.start_file("data/client.lzma", options).unwrap();
        archive.write_all(b"client-patch").unwrap();
        archive.start_file("data/server.lzma", options).unwrap();
        archive.write_all(b"server-patch").unwrap();
        let bytes = archive.finish().unwrap().into_inner();

        let parsed =
            parse_installer(bytes, "https://example.com/installer.jar")
                .unwrap();
        assert_eq!(parsed.artifacts.len(), 2);
        assert_eq!(parsed.profile.processors.as_ref().unwrap().len(), 1);
        assert!(parsed.profile.libraries.iter().any(|library| {
            library.name == "example:processor:1.0"
                && !library.include_in_classpath
        }));
        assert!(parsed.profile.libraries.iter().any(|library| {
            library.name == "example:generated:1.0" && !library.downloadable
        }));
        let data = parsed.profile.data.unwrap();
        assert!(data["BINPATCH"].client.starts_with("[com.axolotl."));
        assert_eq!(
            data[INSTALLER_URL_KEY].client,
            "https://example.com/installer.jar"
        );
    }

    #[test]
    fn natural_version_order_keeps_latest_first() {
        let mut versions = ["55.1.9", "55.1.13", "55.1.10"];
        versions.sort_by(|left, right| compare_versions(right, left));
        assert_eq!(versions, ["55.1.13", "55.1.10", "55.1.9"]);
    }
}
