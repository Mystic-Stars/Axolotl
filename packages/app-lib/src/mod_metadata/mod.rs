//! Parse mod metadata files from inside JAR archives to extract mod identity
//! information (mod ID, name, version, authors, etc.) when Modrinth API
//! lookups fail or have no match.
//!
//! Supported formats:
//! - Fabric: `fabric.mod.json` (JSON)
//! - Quilt: `quilt.mod.json` (JSON, same shape wrapped under `quilt_loader`)
//! - Forge: `META-INF/mods.toml` (TOML)
//! - NeoForge: `META-INF/neoforge.mods.toml` (TOML)
//! - Legacy Forge: `mcmod.info` (JSON array)

mod fabric;
pub mod icon;
pub mod manifest;
mod mcmod_info;
mod toml_mod;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Unified local mod metadata extracted from inside a JAR.
///
/// Only `mod_id` is required; all other fields are best-effort.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModMetadata {
    /// Unique mod identifier (e.g. "sodium", "minecraft")
    pub mod_id: String,
    /// Human-readable display name
    pub name: Option<String>,
    /// Mod version string
    pub version: Option<String>,
    /// Author list
    #[serde(default)]
    pub authors: Vec<String>,
    /// Short description
    pub description: Option<String>,
    /// Website or project URL
    pub url: Option<String>,
    /// Path to icon inside the JAR (e.g. "icon.png" or "assets/.../icon.png")
    pub icon_path: Option<String>,
    /// Supported Minecraft version range (e.g. ">=1.20", "[1.20,1.21)", "1.12.2")
    pub minecraft_version: Option<String>,
    /// Required loader version (e.g. ">=0.15.0", "[52,)")
    pub loader_version: Option<String>,
    /// Loader type (e.g. "fabric", "forge", "neoforge", "quilt")
    pub loader: Option<String>,
    /// Required dependencies declared in the embedded metadata (Fabric
    /// `depends`, Quilt `depends`, Forge mandatory dependencies).
    ///
    /// `None` marks JSON written before dependency extraction existed and
    /// triggers a one-time re-extraction of the file.
    #[serde(default)]
    pub dependencies: Option<Vec<LocalModDependency>>,
}

/// One required dependency declared in embedded mod metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModDependency {
    pub mod_id: String,
    #[serde(default)]
    pub version_range: Option<String>,
}

/// Dependency ids that refer to the runtime environment instead of content
/// that can be linked against installed files.
pub(crate) fn is_env_dependency_id(id: &str) -> bool {
    matches!(
        id,
        "minecraft"
            | "java"
            | "fabricloader"
            | "quilt_loader"
            | "forge"
            | "neoforge"
            | "fml"
    )
}

/// Try to extract `LocalModMetadata` from raw JAR bytes.
///
/// Returns `None` when the JAR does not contain any known mod metadata file
/// or when none of the supported formats can be successfully parsed.
pub fn extract_mod_metadata(bytes: &Bytes) -> Option<LocalModMetadata> {
    let cursor = std::io::Cursor::new(&**bytes);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;

    // Try each known metadata path in priority order.
    if let Some(meta) = try_fabric(&mut archive) {
        return Some(meta);
    }
    if let Some(meta) = try_quilt(&mut archive) {
        return Some(meta);
    }
    if let Some(meta) =
        try_toml_path(&mut archive, "META-INF/neoforge.mods.toml")
    {
        return Some(meta);
    }
    if let Some(meta) = try_toml_path(&mut archive, "META-INF/mods.toml") {
        return Some(meta);
    }
    if let Some(meta) = try_mcmod_info(&mut archive) {
        return Some(meta);
    }

    None
}

/// Validate the parts of embedded mod metadata that can be compared without
/// guessing. Unknown legacy range syntax is intentionally left unverifiable;
/// an explicit contradiction is rejected before the JAR reaches an instance.
pub fn validate_mod_metadata_target(
    bytes: &Bytes,
    minecraft_version: Option<&str>,
    loader: Option<&str>,
) -> Result<LocalModMetadata, String> {
    let metadata = extract_mod_metadata(bytes).ok_or_else(|| {
        "The downloaded mod has no readable loader metadata".to_string()
    })?;
    if metadata.mod_id.trim().is_empty() {
        return Err("The downloaded mod has an empty mod ID".to_string());
    }
    if let Some(expected_loader) = loader {
        if let Some(actual_loader) = metadata.loader.as_deref()
            && !actual_loader.eq_ignore_ascii_case(expected_loader)
        {
            return Err(format!(
                "The downloaded mod declares loader {actual_loader}, expected {expected_loader}"
            ));
        }
    }
    if let (Some(target), Some(range)) =
        (minecraft_version, metadata.minecraft_version.as_deref())
        && minecraft_version_matches_range(target, range) == Some(false)
    {
        return Err(format!(
            "The downloaded mod declares Minecraft range {range}, which excludes {target}"
        ));
    }
    Ok(metadata)
}

fn minecraft_version_matches_range(target: &str, range: &str) -> Option<bool> {
    let range = range.trim();
    if range.is_empty() || range == "*" {
        return Some(true);
    }
    if range.contains("||") {
        let matches = range
            .split("||")
            .filter_map(|part| minecraft_version_matches_range(target, part))
            .collect::<Vec<_>>();
        return (!matches.is_empty())
            .then(|| matches.into_iter().any(|value| value));
    }
    if let Some(version) = range.strip_prefix(">=") {
        return compare_minecraft_versions(target, version.trim())
            .map(|order| order.is_ge());
    }
    if let Some(version) = range.strip_prefix('~') {
        let version = version.trim();
        let parse_components = |value: &str| {
            value
                .trim()
                .strip_prefix('v')
                .unwrap_or(value.trim())
                .split('.')
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
                .ok()
        };
        let target = parse_components(target)?;
        let version = parse_components(version)?;
        return (target.len() >= version.len()).then(|| {
            target
                .iter()
                .zip(version.iter())
                .all(|(target, expected)| target == expected)
        });
    }
    if let Some(inner) = range
        .strip_prefix(['[', '('])
        .and_then(|value| value.strip_suffix([']', ')']))
    {
        let lower_inclusive = range.starts_with('[');
        let upper_inclusive = range.ends_with(']');
        let (lower, upper) = inner.split_once(',')?;
        let lower_matches = if lower.trim().is_empty() {
            true
        } else {
            let order = compare_minecraft_versions(target, lower.trim())?;
            order.is_gt() || (lower_inclusive && order.is_eq())
        };
        let upper_matches = if upper.trim().is_empty() {
            true
        } else {
            let order = compare_minecraft_versions(target, upper.trim())?;
            order.is_lt() || (upper_inclusive && order.is_eq())
        };
        return Some(lower_matches && upper_matches);
    }
    compare_minecraft_versions(target, range).map(|order| order.is_eq())
}

fn compare_minecraft_versions(
    left: &str,
    right: &str,
) -> Option<std::cmp::Ordering> {
    let parse = |value: &str| {
        value
            .trim()
            .strip_prefix('v')
            .unwrap_or(value.trim())
            .split('.')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()
    };
    let left = parse(left)?;
    let right = parse(right)?;
    let width = left.len().max(right.len());
    Some(
        (0..width)
            .map(|index| {
                left.get(index)
                    .copied()
                    .unwrap_or_default()
                    .cmp(&right.get(index).copied().unwrap_or_default())
            })
            .find(|order| !order.is_eq())
            .unwrap_or(std::cmp::Ordering::Equal),
    )
}

// ── format-specific parsers ────────────────────────────────────────────────

fn try_fabric(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
) -> Option<LocalModMetadata> {
    let mut file = archive.by_name("fabric.mod.json").ok()?;
    let parsed: fabric::FabricModJson =
        serde_json::from_reader(&mut file).ok()?;

    let authors = merge_authors(&parsed.authors, &parsed.contributors);
    // A mod without an id cannot be identified; skip it rather than
    // fabricating a shared placeholder that would make mods collide.
    let mod_id = parsed.id.clone()?;

    Some(LocalModMetadata {
        mod_id,
        name: parsed.name,
        version: parsed.version,
        authors,
        description: parsed.description,
        url: extract_contact_url(&parsed._contact),
        icon_path: parsed.icon.as_ref().and_then(|icon| icon.resolve()),
        minecraft_version: fabric::fabric_dep_value(
            &parsed.depends,
            "minecraft",
        ),
        loader_version: fabric::fabric_dep_value(
            &parsed.depends,
            "fabricloader",
        ),
        loader: Some("fabric".into()),
        dependencies: Some(fabric::fabric_dependencies(&parsed.depends)),
    })
}

fn try_quilt(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
) -> Option<LocalModMetadata> {
    let mut file = archive.by_name("quilt.mod.json").ok()?;
    let parsed: fabric::QuiltModJson =
        serde_json::from_reader(&mut file).ok()?;

    let inner = parsed.quilt_loader;
    let authors = merge_authors(&inner.authors, &inner.contributors);
    let mod_id = inner.id.clone()?;

    Some(LocalModMetadata {
        mod_id,
        name: inner.name,
        version: inner.version,
        authors,
        description: inner.description,
        url: extract_contact_url(&inner._contact),
        icon_path: inner.icon.as_ref().and_then(|icon| icon.resolve()),
        minecraft_version: fabric::quilt_dep_value(&inner.depends, "minecraft"),
        loader_version: fabric::quilt_dep_value(&inner.depends, "quilt_loader"),
        loader: Some("quilt".into()),
        dependencies: Some(fabric::quilt_dependencies(&inner.depends)),
    })
}

fn try_toml_path(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    path: &str,
) -> Option<LocalModMetadata> {
    let mut file = archive.by_name(path).ok()?;
    let mut content = String::new();
    std::io::Read::read_to_string(&mut file, &mut content).ok()?;
    let parsed: toml_mod::ModsToml = toml::from_str(&content).ok()?;

    // A mod jar may declare several [[mods]] entries (bundled mods); report
    // the first entry that carries an ID. An id-less entry (e.g. the
    // "minecraft" marker used by some packs) must not discard the metadata
    // of the real mod that follows.
    let entry = parsed
        .mods?
        .into_iter()
        .find(|entry| entry.mod_id.is_some())?;
    let mod_id = entry.mod_id.clone()?;

    let authors: Vec<String> = entry
        .authors
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Determine loader type from the file path.
    let is_neoforge = path.contains("neoforge");
    let loader = if is_neoforge {
        Some("neoforge".into())
    } else {
        Some("forge".into())
    };
    // The root `loaderVersion` IS the Forge/NeoForge loader version.
    let loader_version = parsed.loader_version.clone();

    // Look up dependencies for this mod's modId.
    let minecraft_version = parsed
        .dependencies
        .as_ref()
        .and_then(|deps| deps.get(&mod_id))
        .and_then(|entries| {
            entries
                .iter()
                .find(|dep| dep.mod_id.as_deref() == Some("minecraft"))
                .and_then(|dep| dep.version_range.clone())
        });
    let dependencies = parsed
        .dependencies
        .as_ref()
        .and_then(|deps| deps.get(&mod_id))
        .map(|entries| {
            entries
                .iter()
                .filter(|dep| dep.mandatory.unwrap_or(true))
                .filter_map(|dep| {
                    let dep_id = dep.mod_id.clone()?;
                    if is_env_dependency_id(&dep_id) {
                        return None;
                    }
                    Some(LocalModDependency {
                        mod_id: dep_id,
                        version_range: dep.version_range.clone(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(LocalModMetadata {
        mod_id,
        name: entry.display_name,
        version: entry.version,
        authors,
        description: entry.description,
        url: entry.display_url,
        icon_path: entry.logo_file,
        minecraft_version,
        loader_version,
        loader,
        dependencies: Some(dependencies),
    })
}

fn try_mcmod_info(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
) -> Option<LocalModMetadata> {
    let mut file = archive.by_name("mcmod.info").ok()?;
    let entries: Vec<mcmod_info::McmodInfoEntry> =
        serde_json::from_reader(&mut file).ok()?;

    let entry = entries.into_iter().next()?;
    let mod_id = entry.modid.clone()?;

    Some(LocalModMetadata {
        mod_id,
        name: entry.name,
        version: entry.version,
        authors: entry.authors.unwrap_or_default(),
        description: entry.description,
        url: entry.url,
        icon_path: entry.logo_file,
        minecraft_version: entry.mcversion,
        loader_version: None,
        loader: Some("forge".into()),
        dependencies: Some(Vec::new()),
    })
}

// ── helpers ────────────────────────────────────────────────────────────────

fn merge_authors(
    primary: &[fabric::FabricAuthorOrArray],
    contributors: &[fabric::FabricAuthorOrArray],
) -> Vec<String> {
    primary
        .iter()
        .chain(contributors.iter())
        .filter_map(|author| match author {
            fabric::FabricAuthorOrArray::Plain(s) => Some(s.clone()),
            fabric::FabricAuthorOrArray::Object { name } => name.clone(),
        })
        .collect()
}

/// Extract a URL from Fabric's `contact` object (often has `"homepage"`, `"sources"`, etc.).
fn extract_contact_url(contact: &Option<serde_json::Value>) -> Option<String> {
    let obj = contact.as_ref()?.as_object()?;
    // Prefer homepage, then sources, then any string value.
    if let Some(homepage) = obj.get("homepage").and_then(|v| v.as_str()) {
        return Some(homepage.to_string());
    }
    if let Some(sources) = obj.get("sources").and_then(|v| v.as_str()) {
        return Some(sources.to_string());
    }
    // Fallback: return the first string field found.
    obj.values().find_map(|v| v.as_str().map(String::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn neoforge_jar(minecraft_range: &str) -> Bytes {
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        archive
            .start_file(
                "META-INF/neoforge.mods.toml",
                zip::write::FileOptions::<()>::default(),
            )
            .unwrap();
        archive
            .write_all(
                format!(
                    r#"
modLoader = "javafml"
loaderVersion = "[21,)"

[[mods]]
modId = "fixture"
version = "21.0.65"

[[dependencies.fixture]]
modId = "minecraft"
mandatory = true
versionRange = "{minecraft_range}"
"#
                )
                .as_bytes(),
            )
            .unwrap();
        Bytes::from(archive.finish().unwrap().into_inner())
    }

    #[test]
    fn validates_neoforge_loader_and_minecraft_range() {
        let jar = neoforge_jar("[1.21.1,1.22)");
        assert!(
            validate_mod_metadata_target(
                &jar,
                Some("1.21.1"),
                Some("neoforge"),
            )
            .is_ok()
        );
        assert!(
            validate_mod_metadata_target(&jar, Some("1.22"), Some("neoforge"),)
                .is_err()
        );
        assert!(
            validate_mod_metadata_target(&jar, Some("1.21.1"), Some("forge"),)
                .is_err()
        );
    }

    #[test]
    fn minecraft_range_does_not_match_a_shared_string_prefix() {
        assert_eq!(
            minecraft_version_matches_range("1.210", "~1.21"),
            Some(false)
        );
        assert_eq!(
            minecraft_version_matches_range("1.21.1", "~1.21"),
            Some(true)
        );
    }
}
