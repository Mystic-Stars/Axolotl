use crate::minecraft::{
    Argument, ArgumentType, Library, VersionInfo, VersionType,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// The latest version of the format the fabric model structs deserialize to
pub const CURRENT_FABRIC_FORMAT_VERSION: usize = 0;
/// The latest version of the format the fabric model structs deserialize to
pub const CURRENT_FORGE_FORMAT_VERSION: usize = 0;
/// The latest version of the format the quilt model structs deserialize to
pub const CURRENT_QUILT_FORMAT_VERSION: usize = 1;
/// The latest version of the format the neoforge model structs deserialize to
pub const CURRENT_NEOFORGE_FORMAT_VERSION: usize = 0;

/// Metadata for locating and caching a loader manifest.
#[derive(Debug, Clone)]
pub struct LoaderManifestMetadata {
    /// The canonical loader name used in launcher-meta paths.
    pub loader: String,
    /// The latest manifest format version for this loader.
    pub format_version: usize,
    /// The cache key that includes the loader format version.
    pub cache_key: String,
    /// The launcher-meta path to the manifest.
    pub path: String,
}

/// Returns metadata for the latest manifest format for the provided loader.
pub fn loader_manifest_metadata(loader: &str) -> LoaderManifestMetadata {
    let format_version = current_loader_manifest_format_version(loader);
    let cache_key = format!("{loader}-v{format_version}");
    let path = format!("{loader}/v{format_version}/manifest.json");

    LoaderManifestMetadata {
        loader: loader.to_string(),
        format_version,
        cache_key,
        path,
    }
}

/// Returns loader manifest metadata from a versioned cache key.
pub fn loader_manifest_metadata_from_cache_key(
    cache_key: &str,
) -> LoaderManifestMetadata {
    if let Some((loader, format_version)) =
        cache_key.rsplit_once("-v").and_then(|(loader, version)| {
            version
                .parse::<usize>()
                .ok()
                .map(|version| (loader, version))
        })
    {
        let cache_key = format!("{loader}-v{format_version}");
        let path = format!("{loader}/v{format_version}/manifest.json");

        LoaderManifestMetadata {
            loader: loader.to_string(),
            format_version,
            cache_key,
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
        _ => 0,
    }
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
pub fn normalize_loader_libraries(
    loader: &str,
    game_version: &str,
    libraries: &mut Vec<Library>,
) -> Vec<String> {
    if loader != "fabric" || !uses_unobfuscated_minecraft(game_version) {
        return Vec::new();
    }

    let mut removed = Vec::new();
    libraries.retain(|library| {
        let mut coordinates = library.name.split(':');
        let is_intermediary = coordinates.next() == Some("net.fabricmc")
            && coordinates.next() == Some("intermediary")
            && coordinates.next().is_some();

        if is_intermediary {
            removed.push(library.name.clone());
        }

        !is_intermediary
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

    VersionInfo {
        arguments: if let Some(partial_args) = partial.arguments {
            if let Some(merge_args) = merge.arguments {
                let mut new_map = HashMap::new();

                fn add_keys(
                    new_map: &mut HashMap<ArgumentType, Vec<Argument>>,
                    args: HashMap<ArgumentType, Vec<Argument>>,
                ) {
                    for (type_, arguments) in args {
                        for arg in arguments {
                            if let Some(vec) = new_map.get_mut(&type_) {
                                vec.push(arg);
                            } else {
                                new_map.insert(type_, vec![arg]);
                            }
                        }
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
        },
        asset_index: merge.asset_index,
        assets: merge.assets,
        downloads: merge.downloads,
        id: partial.id.replace(DUMMY_REPLACE_STRING, &merge_id),
        java_version: merge.java_version,
        libraries: libraries
            .into_iter()
            .chain(partial.libraries)
            .map(|mut x| {
                x.name = x.name.replace(DUMMY_REPLACE_STRING, &merge_id);

                x
            })
            .collect::<Vec<_>>(),
        logging: merge.logging,
        main_class: if let Some(main_class) = partial.main_class {
            main_class
        } else {
            merge.main_class
        },
        minecraft_arguments: partial.minecraft_arguments,
        minimum_launcher_version: merge.minimum_launcher_version,
        release_time: partial.release_time,
        time: partial.time,
        type_: partial.type_,
        data: partial.data,
        processors: partial.processors,
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
        assert_eq!(fabric.format_version, 0);
        assert_eq!(fabric.cache_key, "fabric-v0");
        assert_eq!(fabric.path, "fabric/v0/manifest.json");

        assert_eq!(loader_manifest_metadata("quilt").format_version, 1);
        assert_eq!(loader_manifest_metadata("forge").format_version, 0);
        assert_eq!(loader_manifest_metadata("neo").format_version, 0);
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
