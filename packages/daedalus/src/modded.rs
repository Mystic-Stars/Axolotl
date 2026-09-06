use crate::minecraft::{
    Argument, ArgumentType, JavaVersion, Library, VersionInfo, VersionType,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};

/// The latest version of the format the fabric model structs deserialize to
pub const CURRENT_FABRIC_FORMAT_VERSION: usize = 1;
/// The latest version of the format the fabric model structs deserialize to
pub const CURRENT_FORGE_FORMAT_VERSION: usize = 3;
/// The latest version of the format the quilt model structs deserialize to
pub const CURRENT_QUILT_FORMAT_VERSION: usize = 2;
/// The latest version of the format the neoforge model structs deserialize to
pub const CURRENT_NEOFORGE_FORMAT_VERSION: usize = 1;
/// Current OptiFine manifest cache format.
pub const CURRENT_OPTIFINE_FORMAT_VERSION: usize = 1;
/// Current Cleanroom manifest cache format.
pub const CURRENT_CLEANROOM_FORMAT_VERSION: usize = 1;
/// Current LiteLoader manifest cache format.
pub const CURRENT_LITELOADER_FORMAT_VERSION: usize = 2;
/// Current Legacy Fabric manifest cache format.
pub const CURRENT_LEGACY_FABRIC_FORMAT_VERSION: usize = 1;
/// Current Babric manifest cache format.
pub const CURRENT_BABRIC_FORMAT_VERSION: usize = 2;

/// Metadata for locating and caching a loader manifest.
#[derive(Debug, Clone)]
pub struct LoaderManifestMetadata {
    /// The canonical loader name used in launcher-meta paths.
    pub loader: String,
    /// The latest manifest format version for this loader.
    pub format_version: usize,
    /// The cache key that includes the loader format version.
    pub cache_key: String,
    /// Optional Minecraft version for game-scoped loader metadata.
    pub game_version: Option<String>,
    /// The fallback launcher-meta path to the manifest.
    pub path: String,
}

/// Returns metadata for the latest manifest format for the provided loader.
pub fn loader_manifest_metadata(loader: &str) -> LoaderManifestMetadata {
    let format_version = current_loader_manifest_format_version(loader);
    let cache_key = format!("{loader}-v{format_version}");
    let path = fallback_loader_manifest_path(loader);

    LoaderManifestMetadata {
        loader: loader.to_string(),
        format_version,
        cache_key,
        game_version: None,
        path,
    }
}

/// Returns metadata for a loader manifest scoped to one Minecraft version.
pub fn loader_manifest_metadata_for_game(
    loader: &str,
    game_version: &str,
) -> LoaderManifestMetadata {
    let mut metadata = loader_manifest_metadata(loader);
    metadata.cache_key = format!("{}:{game_version}", metadata.cache_key);
    metadata.game_version = Some(game_version.to_string());
    metadata
}

/// Returns loader manifest metadata from a versioned cache key.
pub fn loader_manifest_metadata_from_cache_key(
    cache_key: &str,
) -> LoaderManifestMetadata {
    let (manifest_key, game_version) = cache_key
        .split_once(':')
        .map_or((cache_key, None), |(key, game_version)| {
            (key, Some(game_version.to_string()))
        });
    if let Some((loader, format_version)) = manifest_key
        .rsplit_once("-v")
        .and_then(|(loader, version)| {
            version
                .parse::<usize>()
                .ok()
                .map(|version| (loader, version))
        })
    {
        let path = fallback_loader_manifest_path(loader);

        LoaderManifestMetadata {
            loader: loader.to_string(),
            format_version,
            cache_key: cache_key.to_string(),
            game_version,
            path,
        }
    } else {
        loader_manifest_metadata(cache_key)
    }
}

fn current_loader_manifest_format_version(loader: &str) -> usize {
    match loader {
        "fabric" => CURRENT_FABRIC_FORMAT_VERSION,
        "forge" => CURRENT_FORGE_FORMAT_VERSION,
        "quilt" => CURRENT_QUILT_FORMAT_VERSION,
        "neo" => CURRENT_NEOFORGE_FORMAT_VERSION,
        "optifine" => CURRENT_OPTIFINE_FORMAT_VERSION,
        "cleanroom" => CURRENT_CLEANROOM_FORMAT_VERSION,
        "lite_loader" => CURRENT_LITELOADER_FORMAT_VERSION,
        "legacy_fabric" => CURRENT_LEGACY_FABRIC_FORMAT_VERSION,
        "babric" => CURRENT_BABRIC_FORMAT_VERSION,
        _ => 0,
    }
}

fn fallback_loader_manifest_path(loader: &str) -> String {
    let format_version = match loader {
        "quilt" => 1,
        _ => 0,
    };
    format!("{loader}/v{format_version}/manifest.json")
}

/// The dummy replace string library names, inheritsFrom, and version names should be replaced with
pub const DUMMY_REPLACE_STRING: &str = "${modrinth.gameVersion}";

/// Returns whether a Minecraft version uses the unobfuscated distribution.
pub fn uses_unobfuscated_minecraft(game_version: &str) -> bool {
    if let Some((year, snapshot)) = game_version.split_once('w') {
        let snapshot = snapshot.as_bytes();
        return year.parse::<u32>().is_ok_and(|year| year >= 26)
            && snapshot.len() >= 3
            && snapshot[..2].iter().all(u8::is_ascii_digit)
            && snapshot[2..].iter().all(u8::is_ascii_alphabetic);
    }

    let mut components = game_version.split('.');
    let Some(major) = components
        .next()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    let Some(minor) = components
        .next()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };

    if components.any(|value| value.parse::<u32>().is_err()) {
        return false;
    }

    major > 26 || (major == 26 && minor >= 1)
}

/// Removes loader libraries that are incompatible with the selected game version.
///
/// For Cleanroom the vanilla LWJGL 2 family is dropped: the
/// `org.lwjgl.lwjgl:lwjgl` binding (shares the `org.lwjgl` package with the
/// Cleanroom LWJGL 3 line), the natives-only carrier
/// `org.lwjgl.lwjgl:lwjgl-platform` (whose root-level extracted `lwjgl.dll` /
/// `liblwjgl.so` / `liblwjgl.dylib` would be what `System.loadLibrary("lwjgl")`
/// resolves first in the natives directory, while the Cleanroom LWJGL 3
/// natives are nested), and `org.lwjgl.lwjgl:lwjgl_util` at exactly
/// `2.9.4-nightly-20150209` (LWJGL 2 utility classes the LWJGL 3 port does
/// not provide; the version condition mirrors HMCL's `DefaultLauncher`
/// Cleanroom handling, which removes every classpath entry containing
/// `2.9.4-nightly-20150209`). HMCL itself never classpaths native jars and
/// its first-wins native extraction over self-contained official Cleanroom
/// manifests keeps the old LWJGL 2 native out of the natives directory, so
/// dropping the carrier is an Axolotl-side strengthening for merged
/// direct links, not an HMCL library rule. The vanilla JNA platform and the
/// Mojang ICU bundle are removed by the pre-existing Axolotl Cleanroom rule
/// (not HMCL logic): with Cleanroom's newer JNA line on the classpath the
/// 1.12.2-era `net.java.dev.jna:platform` and `com.ibm.icu:icu4j-core-mojang`
/// carry conflicting runtime-class versions. The Cleanroom LWJGL 3 line
/// (`org.lwjgl:lwjgl:3.4.1-unsafe`), `lwjglxx`, and the vanilla JNA core are
/// untouched.
pub fn normalize_loader_libraries(
    loader: &str,
    game_version: &str,
    libraries: &mut Vec<Library>,
) -> Vec<String> {
    let remove_fabric_intermediary =
        loader == "fabric" && uses_unobfuscated_minecraft(game_version);
    let remove_cleanroom_conflicts = loader == "cleanroom";
    if !remove_fabric_intermediary && !remove_cleanroom_conflicts {
        return Vec::new();
    }

    let mut removed = Vec::new();
    libraries.retain(|library| {
        let mut coordinates = library.name.split(':');
        let group = coordinates.next();
        let artifact = coordinates.next();
        let version = coordinates.next();
        let has_version = version.is_some();
        let is_intermediary = remove_fabric_intermediary
            && group == Some("net.fabricmc")
            && artifact == Some("intermediary")
            && has_version;
        let conflicts_with_cleanroom = remove_cleanroom_conflicts
            && has_version
            && (matches!(
                (group, artifact),
                (Some("org.lwjgl.lwjgl"), Some("lwjgl"))
                    | (Some("org.lwjgl.lwjgl"), Some("lwjgl-platform"))
                    | (Some("net.java.dev.jna"), Some("platform"))
                    | (Some("com.ibm.icu"), Some("icu4j-core-mojang"))
            ) || (group == Some("org.lwjgl.lwjgl")
                && artifact == Some("lwjgl_util")
                && version == Some("2.9.4-nightly-20150209")));

        if is_intermediary || conflicts_with_cleanroom {
            removed.push(library.name.clone());
        }

        !is_intermediary && !conflicts_with_cleanroom
    });
    removed
}

/// A data variable entry that depends on the side of the installation
#[derive(Serialize, Deserialize, Debug)]
pub struct SidedDataEntry {
    /// The value on the client
    pub client: String,
    /// The value on the server
    pub server: String,
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    serde_json::from_str::<DateTime<Utc>>(&format!("\"{s}\""))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|date| date.and_utc())
        })
        .map_err(serde::de::Error::custom)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// A partial version returned by fabric meta
pub struct PartialVersionInfo {
    /// The version ID of the version
    pub id: String,
    /// The version ID this partial version inherits from
    pub inherits_from: String,
    /// The time that the version was released
    #[serde(deserialize_with = "deserialize_date")]
    pub release_time: DateTime<Utc>,
    /// The latest time a file in this version was updated
    #[serde(deserialize_with = "deserialize_date")]
    pub time: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The classpath to the main class to launch the game
    pub main_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// (Legacy) Arguments passed to the game
    pub minecraft_arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Arguments passed to the game or JVM
    pub arguments: Option<HashMap<ArgumentType, Vec<Argument>>>,
    /// Libraries that the version depends on
    pub libraries: Vec<Library>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Java runtime required by the loader profile, when it overrides Minecraft.
    pub java_version: Option<JavaVersion>,
    #[serde(rename = "type")]
    /// The type of version
    pub type_: VersionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// (Forge-only)
    pub data: Option<HashMap<String, SidedDataEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// (Forge-only) The list of processors to run after downloading the files
    pub processors: Option<Vec<Processor>>,
}

/// A processor to be ran after downloading the files
#[derive(Serialize, Deserialize, Debug)]
pub struct Processor {
    /// Maven coordinates for the JAR library of this processor.
    pub jar: String,
    /// Maven coordinates for all the libraries that must be included in classpath when running this processor.
    pub classpath: Vec<String>,
    /// Arguments for this processor.
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Represents a map of outputs. Keys and values can be data values
    pub outputs: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Which sides this processor shall be ran on.
    /// Valid values: client, server, extract
    pub sides: Option<Vec<String>>,
}

/// Merges a partial version into a complete one
pub fn merge_partial_version(
    partial: PartialVersionInfo,
    merge: VersionInfo,
) -> VersionInfo {
    let merge_id = merge.id.clone();

    let mut libraries = vec![];

    // We skip duplicate libraries that exist already in the partial version
    for mut lib in merge.libraries {
        let lib_artifact = lib.name.rsplit_once(':').map(|x| x.0);

        if let Some(lib_artifact) = lib_artifact {
            if !partial.libraries.iter().any(|x| {
                let target_artifact = x.name.rsplit_once(':').map(|x| x.0);

                target_artifact == Some(lib_artifact) && x.include_in_classpath
            }) {
                libraries.push(lib);
            } else {
                lib.include_in_classpath = false;
            }
        } else {
            libraries.push(lib);
        }
    }

    let arguments = if let Some(partial_args) = partial.arguments {
        if let Some(merge_args) = merge.arguments {
            let mut new_map = HashMap::new();

            fn argument_key(argument: &Argument) -> String {
                serde_json::to_string(argument)
                    .unwrap_or_else(|_| format!("{argument:?}"))
            }

            fn add_keys(
                new_map: &mut HashMap<ArgumentType, Vec<Argument>>,
                args: HashMap<ArgumentType, Vec<Argument>>,
            ) {
                for (type_, arguments) in args {
                    let existing = new_map.entry(type_).or_default();
                    let max_overlap = existing.len().min(arguments.len());
                    let overlap = (1..=max_overlap)
                        .rev()
                        .find(|&length| {
                            existing[existing.len() - length..]
                                .iter()
                                .map(argument_key)
                                .eq(arguments[..length]
                                    .iter()
                                    .map(argument_key))
                        })
                        .unwrap_or(0);
                    existing.extend(arguments.into_iter().skip(overlap));
                }
            }

            add_keys(&mut new_map, merge_args);
            add_keys(&mut new_map, partial_args);

            Some(new_map)
        } else {
            Some(partial_args)
        }
    } else {
        merge.arguments
    };
    let mut libraries = libraries
        .into_iter()
        .chain(partial.libraries)
        .map(|mut library| {
            library.name =
                library.name.replace(DUMMY_REPLACE_STRING, &merge_id);
            library
        })
        .collect::<Vec<_>>();
    // Loader profiles may repeat a vanilla coordinate with a reduced
    // description (Forge 1.16.x is a notable example: its duplicate LWJGL
    // entry omits `natives` and `downloads.classifiers`). Merge metadata from
    // duplicates before dropping them so native archives remain discoverable.
    let mut merged_libraries = Vec::with_capacity(libraries.len());
    for library in libraries {
        if let Some(existing) = merged_libraries
            .iter_mut()
            .find(|existing: &&mut Library| existing.name == library.name)
        {
            existing.include_in_classpath |= library.include_in_classpath;
            if existing.natives.is_none() {
                existing.natives = library.natives.clone();
            }
            if let Some(downloads) = &library.downloads {
                let existing_downloads =
                    existing.downloads.get_or_insert_with(|| downloads.clone());
                if existing_downloads.artifact.is_none() {
                    existing_downloads.artifact = downloads.artifact.clone();
                }
                if existing_downloads.classifiers.is_none() {
                    existing_downloads.classifiers =
                        downloads.classifiers.clone();
                }
            }
            if existing.extract.is_none() {
                existing.extract = library.extract.clone();
            }
        } else {
            merged_libraries.push(library);
        }
    }
    libraries = merged_libraries;

    VersionInfo {
        arguments,
        asset_index: merge.asset_index,
        assets: merge.assets,
        downloads: merge.downloads,
        id: partial.id.replace(DUMMY_REPLACE_STRING, &merge_id),
        java_version: partial.java_version.or(merge.java_version),
        libraries,
        logging: merge.logging,
        main_class: if let Some(main_class) = partial.main_class {
            main_class
        } else {
            merge.main_class
        },
        minecraft_arguments: partial
            .minecraft_arguments
            .or(merge.minecraft_arguments),
        minimum_launcher_version: merge.minimum_launcher_version,
        release_time: partial.release_time,
        time: partial.time,
        type_: partial.type_,
        data: partial.data,
        processors: partial.processors,
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn library(name: &str) -> Library {
        Library {
            downloads: None,
            extract: None,
            name: name.to_string(),
            url: None,
            natives: None,
            rules: None,
            checksums: None,
            include_in_classpath: true,
            downloadable: true,
        }
    }

    fn version_info() -> VersionInfo {
        VersionInfo {
            arguments: Some(HashMap::from([(
                ArgumentType::Game,
                vec![Argument::Normal("--demo".to_string())],
            )])),
            asset_index: serde_json::from_value(serde_json::json!({
                "id": "legacy",
                "sha1": "test",
                "size": 0,
                "totalSize": 0,
                "url": "https://example.com/index.json"
            }))
            .unwrap(),
            assets: "legacy".to_string(),
            downloads: HashMap::new(),
            id: "1.12.2-forge".to_string(),
            java_version: None,
            libraries: vec![
                library("example:shared:1.0"),
                library("example:base:1.0"),
            ],
            logging: None,
            main_class: "example.Main".to_string(),
            minecraft_arguments: Some(
                "--username ${auth_player_name} --gameDir ${game_directory}"
                    .to_string(),
            ),
            minimum_launcher_version: 0,
            release_time: Utc::now(),
            time: Utc::now(),
            type_: VersionType::Release,
            data: None,
            processors: None,
        }
    }

    #[test]
    fn merge_partial_version_keeps_loader_order_and_removes_exact_duplicates() {
        let now = Utc::now();
        let partial = PartialVersionInfo {
            id: "1.12.2-liteloader".to_string(),
            inherits_from: "1.12.2".to_string(),
            release_time: now,
            time: now,
            main_class: Some("net.minecraft.launchwrapper.Launch".to_string()),
            minecraft_arguments: None,
            arguments: Some(HashMap::from([(
                ArgumentType::Game,
                vec![
                    Argument::Normal("--demo".to_string()),
                    Argument::Normal("--tweakClass".to_string()),
                    Argument::Normal("example.Tweaker".to_string()),
                ],
            )])),
            libraries: vec![
                library("example:shared:2.0"),
                library("example:lite:1.0"),
            ],
            java_version: None,
            type_: VersionType::Release,
            data: None,
            processors: None,
        };

        let merged = merge_partial_version(partial, version_info());
        assert_eq!(
            merged
                .libraries
                .iter()
                .map(|library| library.name.as_str())
                .collect::<Vec<_>>(),
            ["example:base:1.0", "example:shared:2.0", "example:lite:1.0"]
        );
        assert_eq!(merged.main_class, "net.minecraft.launchwrapper.Launch");
        assert_eq!(
            merged.minecraft_arguments.as_deref(),
            Some("--username ${auth_player_name} --gameDir ${game_directory}")
        );
        assert_eq!(
            merged.arguments.unwrap()[&ArgumentType::Game]
                .iter()
                .filter_map(|argument| match argument {
                    Argument::Normal(value) => Some(value.as_str()),
                    Argument::Ruled { .. } => None,
                })
                .collect::<Vec<_>>(),
            ["--demo", "--tweakClass", "example.Tweaker"]
        );
    }

    #[test]
    fn merge_partial_version_preserves_repeated_jvm_options() {
        let now = Utc::now();
        let mut minecraft = version_info();
        minecraft.arguments = Some(HashMap::from([(
            ArgumentType::Jvm,
            vec![Argument::Normal("-cp".to_string())],
        )]));
        let partial = PartialVersionInfo {
            id: "1.20.1-forge".to_string(),
            inherits_from: "1.20.1".to_string(),
            release_time: now,
            time: now,
            main_class: Some(
                "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string(),
            ),
            minecraft_arguments: None,
            arguments: Some(HashMap::from([(
                ArgumentType::Jvm,
                vec![
                    Argument::Normal("--add-opens".to_string()),
                    Argument::Normal(
                        "java.base/java.util.jar=cpw.mods.securejarhandler"
                            .to_string(),
                    ),
                    Argument::Normal("--add-opens".to_string()),
                    Argument::Normal(
                        "java.base/java.lang.invoke=cpw.mods.securejarhandler"
                            .to_string(),
                    ),
                ],
            )])),
            libraries: Vec::new(),
            java_version: None,
            type_: VersionType::Release,
            data: None,
            processors: None,
        };

        let merged = merge_partial_version(partial, minecraft);
        let arguments = &merged.arguments.unwrap()[&ArgumentType::Jvm];
        assert_eq!(
            arguments
                .iter()
                .filter(|argument| {
                    matches!(argument, Argument::Normal(value) if value == "--add-opens")
                })
                .count(),
            2
        );
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// A manifest containing information about a mod loader's versions
pub struct Manifest {
    /// The game versions the mod loader supports
    pub game_versions: Vec<Version>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Groups of game versions that share compatible loader version profiles
    pub version_groups: Vec<VersionGroup>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
///  A game version of Minecraft
pub struct Version {
    /// The minecraft version ID
    pub id: String,
    /// Whether the release is stable or not
    pub stable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The loader profile group for this Minecraft version
    pub version_group: Option<String>,
    /// A map that contains loader versions for the game version
    pub loaders: Vec<LoaderVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// A group of Minecraft versions that share loader version profiles
pub struct VersionGroup {
    /// The version group ID
    pub id: String,
    /// The loader versions for this version group
    pub loaders: Vec<LoaderVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// A version of a Minecraft mod loader
pub struct LoaderVersion {
    /// The version ID of the loader
    pub id: String,
    /// The URL of the version's manifest
    pub url: String,
    /// Whether the loader is stable or not
    pub stable: bool,
    /// How the version profile at `url` should be resolved.
    #[serde(default, skip_serializing_if = "is_json_profile_source")]
    pub profile_source: LoaderProfileSource,
    /// JSON profile used if the official source cannot be resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_url: Option<String>,
}

/// Source format used to resolve a loader version profile.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum LoaderProfileSource {
    /// A partial Minecraft version JSON document.
    #[default]
    Json,
    /// An official Forge-compatible installer JAR.
    Installer,
    /// A profile synthesized from the LiteLoader versions manifest.
    LiteLoader,
    /// A profile synthesized from Babric's Glass Maven metadata and legacy profile.
    Babric,
}

fn is_json_profile_source(source: &LoaderProfileSource) -> bool {
    *source == LoaderProfileSource::Json
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn library(name: &str) -> Library {
        Library {
            downloads: None,
            extract: None,
            name: name.to_string(),
            url: None,
            natives: None,
            rules: None,
            checksums: None,
            include_in_classpath: true,
            downloadable: true,
        }
    }

    #[test]
    fn loader_manifest_metadata_uses_independent_format_versions() {
        let fabric = loader_manifest_metadata("fabric");
        assert_eq!(fabric.format_version, 1);
        assert_eq!(fabric.cache_key, "fabric-v1");
        assert_eq!(fabric.game_version, None);
        assert_eq!(fabric.path, "fabric/v0/manifest.json");

        assert_eq!(loader_manifest_metadata("quilt").format_version, 2);
        assert_eq!(loader_manifest_metadata("forge").format_version, 3);
        assert_eq!(loader_manifest_metadata("neo").format_version, 1);
        assert_eq!(loader_manifest_metadata("babric").format_version, 2);
        assert_eq!(loader_manifest_metadata("babric").cache_key, "babric-v2");
        assert_eq!(
            loader_manifest_metadata("quilt").path,
            "quilt/v1/manifest.json"
        );

        let forge_1201 = loader_manifest_metadata_for_game("forge", "1.20.1");
        let forge_262 = loader_manifest_metadata_for_game("forge", "26.2");
        assert_eq!(forge_1201.cache_key, "forge-v3:1.20.1");
        assert_eq!(forge_262.cache_key, "forge-v3:26.2");
        assert_ne!(forge_1201.cache_key, forge_262.cache_key);
        assert_eq!(forge_1201.game_version.as_deref(), Some("1.20.1"));
        assert_eq!(
            loader_manifest_metadata_from_cache_key(&forge_1201.cache_key)
                .game_version
                .as_deref(),
            Some("1.20.1")
        );
    }

    #[test]
    fn unobfuscated_version_boundary_is_conservative() {
        for version in ["26.1", "26.2", "26.1.1", "27.0", "26w14a"] {
            assert!(uses_unobfuscated_minecraft(version), "{version}");
        }

        for version in [
            "26.0", "26", "1.21", "1.21.11", "25w46a", "26w非", "invalid",
        ] {
            assert!(!uses_unobfuscated_minecraft(version), "{version}");
        }
    }

    #[test]
    fn normalization_only_removes_exact_fabric_intermediary_coordinates() {
        let retained = [
            "net.fabricmc:fabric-loader:0.19.3",
            "net.fabricmc:tiny-mappings-parser:0.3.0",
            "example:intermediary-helper:1.0",
            "example:contains-intermediary:1.0",
        ];
        let mut libraries =
            std::iter::once(library("net.fabricmc:intermediary:26.2"))
                .chain(retained.iter().map(|name| library(name)))
                .collect();

        assert_eq!(
            normalize_loader_libraries("fabric", "26.2", &mut libraries),
            vec!["net.fabricmc:intermediary:26.2"]
        );
        assert_eq!(
            libraries
                .iter()
                .map(|library| library.name.as_str())
                .collect::<Vec<_>>(),
            retained
        );
        assert!(
            normalize_loader_libraries("fabric", "26.2", &mut libraries)
                .is_empty()
        );
        let paths = libraries
            .iter()
            .map(|library| {
                crate::get_path_from_artifact(&library.name).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(!paths.iter().any(|path| {
            path == "net/fabricmc/intermediary/26.2/intermediary-26.2.jar"
        }));
        assert!(paths.iter().any(|path| {
            path == "net/fabricmc/fabric-loader/0.19.3/fabric-loader-0.19.3.jar"
        }));
    }

    #[test]
    fn normalization_removes_intermediary_at_26_1_boundary() {
        let mut libraries = vec![library("net.fabricmc:intermediary:26.1")];
        assert_eq!(
            normalize_loader_libraries("fabric", "26.1", &mut libraries),
            vec!["net.fabricmc:intermediary:26.1"]
        );
        assert!(libraries.is_empty());
    }

    #[test]
    fn normalization_preserves_legacy_fabric_and_other_loaders() {
        for (loader, version) in [
            ("fabric", "1.21"),
            ("quilt", "26.2"),
            ("vanilla", "26.2"),
            ("forge", "26.2"),
            ("neo", "26.2"),
        ] {
            let mut libraries = vec![library("net.fabricmc:intermediary:26.2")];
            assert!(
                normalize_loader_libraries(loader, version, &mut libraries)
                    .is_empty()
            );
            assert_eq!(libraries.len(), 1);
        }
    }

    #[test]
    fn cleanroom_normalization_removes_lwjgl2_family_and_legacy_mojang_conflicts()
     {
        // The vanilla LWJGL 2 family (binding, util at 2.9.4-nightly-20150209,
        // natives carrier at any version) plus the pre-existing Axolotl
        // Cleanroom rule targets (vanilla JNA platform, Mojang ICU) are
        // removed. The Cleanroom LWJGL 3 line, lwjglxx, the version-scoped
        // lwjgl_util at other versions, and the Cleanroom JNA core stay.
        let retained = [
            "org.lwjgl:lwjgl:3.4.1-unsafe",
            "org.lwjgl.lwjgl:lwjgl_util:2.9.2-nightly-20140822",
            "net.java.dev.jna:jna:5.19.1",
            "net.java.dev.jna:jna-platform:5.19.1",
            "com.ibm.icu:icu4j:78.3",
            "com.cleanroommc:lwjglxx:1.1.22",
        ];
        let removed = [
            "org.lwjgl.lwjgl:lwjgl:2.9.4-nightly-20150209",
            "org.lwjgl.lwjgl:lwjgl_util:2.9.4-nightly-20150209",
            "org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209",
            "net.java.dev.jna:platform:3.4.0",
            "com.ibm.icu:icu4j-core-mojang:51.2",
            "org.lwjgl.lwjgl:lwjgl-platform:2.9.2-nightly-20140822",
        ];
        let mut libraries = removed
            .iter()
            .chain(retained.iter())
            .map(|name| library(name))
            .collect();

        assert_eq!(
            normalize_loader_libraries("cleanroom", "1.12.2", &mut libraries),
            removed
        );
        assert_eq!(
            libraries
                .iter()
                .map(|library| library.name.as_str())
                .collect::<Vec<_>>(),
            retained
        );
    }

    #[test]
    fn cleanroom_normalization_matches_lwjgl2_coordinates_by_version_scope() {
        // lwjgl and lwjgl-platform keep the pre-existing artifact match (any
        // version); lwjgl_util only matches at 2.9.4-nightly-20150209,
        // mirroring HMCL's classpath filter for Cleanroom. The LWJGL 3 line
        // and lwjglxx never match.
        let mut libraries = vec![
            library("org.lwjgl.lwjgl:lwjgl:2.9.4-nightly-20150209"),
            library("org.lwjgl.lwjgl:lwjgl_util:2.9.4-nightly-20150209"),
            library("org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209"),
            library("org.lwjgl.lwjgl:lwjgl:2.9.2-nightly-20140822"),
            library("org.lwjgl.lwjgl:lwjgl-platform:2.9.2-nightly-20140822"),
            library("org.lwjgl.lwjgl:lwjgl_util:2.9.2-nightly-20140822"),
            library("org.lwjgl:lwjgl:3.4.1-unsafe"),
            library("org.lwjgl:lwjgl-platform:9.9.9"),
            library("com.cleanroommc:lwjglxx:1.1.22"),
        ];

        assert_eq!(
            normalize_loader_libraries("cleanroom", "1.12.2", &mut libraries),
            [
                "org.lwjgl.lwjgl:lwjgl:2.9.4-nightly-20150209",
                "org.lwjgl.lwjgl:lwjgl_util:2.9.4-nightly-20150209",
                "org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209",
                "org.lwjgl.lwjgl:lwjgl:2.9.2-nightly-20140822",
                "org.lwjgl.lwjgl:lwjgl-platform:2.9.2-nightly-20140822",
            ]
        );
        assert_eq!(
            libraries
                .iter()
                .map(|library| library.name.as_str())
                .collect::<Vec<_>>(),
            [
                "org.lwjgl.lwjgl:lwjgl_util:2.9.2-nightly-20140822",
                "org.lwjgl:lwjgl:3.4.1-unsafe",
                "org.lwjgl:lwjgl-platform:9.9.9",
                "com.cleanroommc:lwjglxx:1.1.22",
            ]
        );
    }

    #[test]
    fn cleanroom_runtime_conflicts_remain_for_other_loaders() {
        for loader in ["vanilla", "forge", "fabric", "quilt", "neo"] {
            let mut libraries = vec![
                library("org.lwjgl.lwjgl:lwjgl:2.9.4-nightly-20150209"),
                library(
                    "org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209",
                ),
                library("net.java.dev.jna:platform:3.4.0"),
                library("com.ibm.icu:icu4j-core-mojang:51.2"),
            ];

            assert!(
                normalize_loader_libraries(loader, "1.12.2", &mut libraries)
                    .is_empty()
            );
            assert_eq!(libraries.len(), 4);
        }
    }

    fn merged_fabric_version(game_version: &str) -> VersionInfo {
        let partial: PartialVersionInfo = serde_json::from_value(json!({
            "id": "fabric-loader-0.19.3-${modrinth.gameVersion}",
            "inheritsFrom": "${modrinth.gameVersion}",
            "releaseTime": "2026-07-22T00:00:00Z",
            "time": "2026-07-22T00:00:00Z",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                { "name": "net.fabricmc:intermediary:${modrinth.gameVersion}" },
                { "name": "net.fabricmc:fabric-loader:0.19.3" },
                { "name": "org.ow2.asm:asm:9.9" }
            ],
            "type": "release"
        }))
        .unwrap();
        let minecraft: VersionInfo = serde_json::from_value(json!({
            "arguments": {},
            "assetIndex": {
                "id": game_version,
                "sha1": "asset-sha1",
                "size": 1,
                "totalSize": 1,
                "url": "https://example.com/assets.json"
            },
            "assets": game_version,
            "downloads": {},
            "id": game_version,
            "libraries": [],
            "mainClass": "net.minecraft.client.main.Main",
            "minimumLauncherVersion": 0,
            "releaseTime": "2026-07-22T00:00:00Z",
            "time": "2026-07-22T00:00:00Z",
            "type": "release"
        }))
        .unwrap();

        merge_partial_version(partial, minecraft)
    }

    #[test]
    fn merged_fabric_v0_profile_is_normalized_before_consumers() {
        let mut modern = merged_fabric_version("26.2");
        assert!(modern
            .libraries
            .iter()
            .any(|library| library.name == "net.fabricmc:intermediary:26.2"));

        assert_eq!(
            normalize_loader_libraries("fabric", "26.2", &mut modern.libraries),
            vec!["net.fabricmc:intermediary:26.2"]
        );
        let serialized = serde_json::to_string(&modern).unwrap();
        assert!(!serialized.contains("net.fabricmc:intermediary"));
        assert!(serialized.contains("net.fabricmc:fabric-loader:0.19.3"));
        assert!(serialized.contains("org.ow2.asm:asm:9.9"));

        let mut legacy = merged_fabric_version("1.21");
        assert!(
            normalize_loader_libraries("fabric", "1.21", &mut legacy.libraries)
                .is_empty()
        );
        assert!(legacy
            .libraries
            .iter()
            .any(|library| library.name == "net.fabricmc:intermediary:1.21"));
    }
}
