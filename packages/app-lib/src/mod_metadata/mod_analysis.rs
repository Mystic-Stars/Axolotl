//! Determine which logical sides (client and/or server) a mod supports.
//!
//! This complements [`crate::mod_metadata`] (which extracts a mod's identity)
//! by answering the side-support question the server installer cares about:
//! "can this mod be installed on a dedicated server?".
//!
//! The analysis is loader-agnostic. A mod may be supplied as a `.jar` archive
//! or as an already-extracted directory, and the work is dispatched to a
//! [`ModDetector`] based on the detected format (Fabric, Forge, ...).
//!
//! # Adding a new loader (Quilt, NeoForge, ...)
//!
//! 1. Add a variant to [`ModType`].
//! 2. Add a detector struct implementing [`ModDetector`] (see
//!    [`FabricDetector`] / [`ForgeDetector`] for the pattern).
//! 3. Register it in [`ModAnalyzer::new`].
//!
//! # Server integration
//!
//! The Fabric loader enforces the `environment` field as a hard rule: a mod
//! declaring `"client"` is refused on a dedicated server, while `"*"` / `"server"`
//! load fine. [`environment_is_client_only`] centralizes that exact rule so the
//! server modpack pruner and this analyzer never drift apart.

use std::io::Read;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mod_metadata::extract_mod_metadata;
use crate::mod_metadata::is_env_dependency_id;
use crate::mod_metadata::LocalModDependency;

/// Errors produced while locating, opening, or parsing a mod file.
#[derive(Debug, Error)]
pub enum ModAnalysisError {
    /// An I/O error occurred while accessing the mod file at `path`.
    #[error("failed to access {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The file is not a valid zip archive (required for `.jar` mods).
    #[error("the file is not a valid jar/zip archive: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Mod metadata could not be parsed as JSON (e.g. `fabric.mod.json`).
    #[error("failed to parse JSON mod metadata: {0}")]
    Json(#[from] serde_json::Error),

    /// Mod metadata could not be parsed as TOML (e.g. Forge `mods.toml`).
    #[error("failed to parse TOML mod metadata: {0}")]
    Toml(#[from] toml::de::Error),

    /// No registered detector recognized the file as a known mod format.
    #[error("the file is not a recognized mod (unknown format)")]
    UnrecognizedFormat,
}

/// Which logical sides a mod is capable of running on.
///
/// Minecraft separates the *client* (the game rendered on a player's machine)
/// from the *server* (the dedicated/integrated server). A mod may support one
/// or both of these sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SideSupport {
    /// The mod can be installed and run on the client.
    pub client: bool,
    /// The mod can be installed and run on the server.
    pub server: bool,
}

impl SideSupport {
    /// Neither side is known to be supported (not yet analyzed / unknown format).
    pub const fn unknown() -> Self {
        Self {
            client: false,
            server: false,
        }
    }

    /// Supports both the client and the server.
    pub const fn both() -> Self {
        Self {
            client: true,
            server: true,
        }
    }

    /// Supports only the client.
    pub const fn client_only() -> Self {
        Self {
            client: true,
            server: false,
        }
    }

    /// Supports only the server.
    pub const fn server_only() -> Self {
        Self {
            client: false,
            server: true,
        }
    }

    /// Whether this mod supports the given [`Side`].
    pub const fn supports(&self, side: Side) -> bool {
        match side {
            Side::Client => self.client,
            Side::Server => self.server,
        }
    }

    /// True when the mod supports both sides.
    pub const fn is_universal(&self) -> bool {
        self.client && self.server
    }

    /// True when the mod runs only on the client.
    pub const fn is_client_only(&self) -> bool {
        self.client && !self.server
    }

    /// True when the mod runs only on the server.
    pub const fn is_server_only(&self) -> bool {
        !self.client && self.server
    }

    /// True when support could not be determined.
    pub const fn is_unknown(&self) -> bool {
        !self.client && !self.server
    }
}

/// A single logical side of Minecraft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Client,
    Server,
}

/// High-level classification of a mod derived from its [`SideSupport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModEnvironment {
    /// Runs only on the client (e.g. HUD/rendering mods like Jade).
    Client,
    /// Runs only on the server (e.g. server administration mods).
    Server,
    /// Runs on both the client and the server.
    Universal,
    /// Side support could not be determined.
    #[default]
    Unknown,
}

impl ModEnvironment {
    /// Derive the classification from a [`SideSupport`].
    pub const fn from_side_support(support: SideSupport) -> Self {
        match (support.client, support.server) {
            (true, false) => ModEnvironment::Client,
            (false, true) => ModEnvironment::Server,
            (true, true) => ModEnvironment::Universal,
            (false, false) => ModEnvironment::Unknown,
        }
    }
}

/// The mod loader / packaging format a mod targets.
///
/// This is the first thing a [`ModDetector`] recognizes. New loaders (Quilt,
/// NeoForge, ...) can be added here without touching any caller code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModType {
    /// Fabric (and Quilt-compatible) mods identified by `fabric.mod.json`.
    Fabric,
    /// Forge / NeoForge mods identified by `META-INF/mods.toml` / `mcmod.info`.
    Forge,
    /// The format could not be recognized by any registered detector.
    #[default]
    Unknown,
}

/// The outcome of analyzing a single mod file.
///
/// Fully serializable so it can cross process / FFI boundaries if needed by the
/// server component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModAnalysis {
    /// The recognized mod loader / format.
    pub mod_type: ModType,

    /// Which logical sides (client/server) this mod supports.
    pub side_support: SideSupport,

    /// Mod identifier from the mod's metadata (e.g. Fabric `id`), if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mod_id: Option<String>,

    /// Human-readable mod name, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Mod version string, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Hard dependencies declared in the embedded metadata (Fabric `depends`,
    /// Forge mandatory `[[dependencies]]`). Used by the server pruner to cascade
    /// removals: dropping a client-only mod also drops mods that require it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<LocalModDependency>,
}

impl ModAnalysis {
    /// Construct an analysis for an unrecognized file (no detector matched).
    pub fn unrecognized() -> Self {
        Self {
            mod_type: ModType::Unknown,
            side_support: SideSupport::unknown(),
            mod_id: None,
            name: None,
            version: None,
            dependencies: Vec::new(),
        }
    }

    /// High-level classification derived from [`ModAnalysis::side_support`].
    pub fn environment(&self) -> ModEnvironment {
        ModEnvironment::from_side_support(self.side_support)
    }

    /// Whether this mod can run on a dedicated server.
    pub fn supports_server(&self) -> bool {
        self.side_support.server
    }

    /// Whether this mod can run on a client.
    pub fn supports_client(&self) -> bool {
        self.side_support.client
    }
}

/// Read-only access to the entries contained inside a mod, independent of
/// whether the mod is shipped as a `.jar` archive or an extracted directory.
///
/// Detectors use this trait so they never need to know how the mod is stored on
/// disk. This keeps analysis loader-agnostic and trivial to test.
pub trait ModFileAccess {
    /// Read the full contents of the entry at `path` (e.g. `fabric.mod.json`).
    ///
    /// Returns `Ok(None)` when no such entry exists.
    fn read_entry(&self, path: &str) -> Result<Option<Vec<u8>>, ModAnalysisError>;

    /// List every entry name contained in the mod (used for format detection).
    ///
    /// Entry names use `/` as the separator (matching jar layout) so detectors
    /// can match paths consistently regardless of the underlying storage.
    fn list_entries(&self) -> Result<Vec<String>, ModAnalysisError>;
}

/// A mod located on disk, either as a `.jar` archive or an extracted folder.
#[derive(Debug)]
pub enum ModFile {
    Jar(JarModFile),
    Dir(DirModFile),
}

impl ModFile {
    /// Open a mod from a filesystem path.
    ///
    /// Directories are treated as already-extracted mods; everything else is
    /// treated as a jar archive.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ModAnalysisError> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(|source| ModAnalysisError::Io {
            path: path.display().to_string(),
            source,
        })?;

        if metadata.is_dir() {
            Ok(Self::Dir(DirModFile::open(path)))
        } else {
            Ok(Self::Jar(JarModFile::open(path)))
        }
    }
}

impl ModFileAccess for ModFile {
    fn read_entry(&self, path: &str) -> Result<Option<Vec<u8>>, ModAnalysisError> {
        match self {
            Self::Jar(jar) => jar.read_entry(path),
            Self::Dir(dir) => dir.read_entry(path),
        }
    }

    fn list_entries(&self) -> Result<Vec<String>, ModAnalysisError> {
        match self {
            Self::Jar(jar) => jar.list_entries(),
            Self::Dir(dir) => dir.list_entries(),
        }
    }
}

/// A mod packaged as a `.jar` (zip) archive.
#[derive(Debug)]
pub struct JarModFile {
    path: PathBuf,
}

impl JarModFile {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl ModFileAccess for JarModFile {
    fn read_entry(&self, path: &str) -> Result<Option<Vec<u8>>, ModAnalysisError> {
        let file = std::fs::File::open(&self.path).map_err(|source| ModAnalysisError::Io {
            path: self.path.display().to_string(),
            source,
        })?;
        let mut archive = zip::ZipArchive::new(file)?;

        match archive.by_name(path) {
            Ok(mut entry) => {
                let mut buf = Vec::with_capacity(entry.size() as usize);
                entry
                    .read_to_end(&mut buf)
                    .map_err(|source| ModAnalysisError::Io {
                        path: path.to_string(),
                        source,
                    })?;
                Ok(Some(buf))
            }
            // Missing entry is not an error: detectors simply won't match.
            Err(zip::result::ZipError::FileNotFound) => Ok(None),
            Err(source) => Err(source.into()),
        }
    }

    fn list_entries(&self) -> Result<Vec<String>, ModAnalysisError> {
        let file = std::fs::File::open(&self.path).map_err(|source| ModAnalysisError::Io {
            path: self.path.display().to_string(),
            source,
        })?;
        let archive = zip::ZipArchive::new(file)?;
        Ok(archive.file_names().map(|name| name.to_string()).collect())
    }
}

/// A mod that has already been extracted into a directory.
#[derive(Debug)]
pub struct DirModFile {
    path: PathBuf,
}

impl DirModFile {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Recursively collect relative entry paths using `/` as the separator,
    /// matching the layout used inside a jar archive.
    fn walk(&self, base: &Path, current: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            let rel = entry
                .path()
                .strip_prefix(base)
                .unwrap_or_else(|_| Path::new(""))
                .to_string_lossy()
                .replace('\\', "/");
            if meta.is_dir() {
                self.walk(base, &entry.path(), out)?;
            } else {
                out.push(rel);
            }
        }
        Ok(())
    }
}

impl ModFileAccess for DirModFile {
    fn read_entry(&self, path: &str) -> Result<Option<Vec<u8>>, ModAnalysisError> {
        let full = self.path.join(path);
        match std::fs::read(&full) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ModAnalysisError::Io {
                path: full.display().to_string(),
                source,
            }),
        }
    }

    fn list_entries(&self) -> Result<Vec<String>, ModAnalysisError> {
        let mut out = Vec::new();
        self.walk(&self.path, &self.path, &mut out)
            .map_err(|source| ModAnalysisError::Io {
                path: self.path.display().to_string(),
                source,
            })?;
        Ok(out)
    }
}

/// A detector recognizes one mod format (Fabric, Forge, ...) and extracts its
/// side-support information.
///
/// [`ModAnalyzer`] owns a list of detectors and asks each one, in turn, whether
/// it [`matches`](ModDetector::matches) a given file. The first detector that
/// matches is used to [`analyze`](ModDetector::analyze) the file.
///
/// `matches` should be cheap (only check for a defining marker such as a
/// metadata file's existence); `analyze` does the actual parsing.
pub trait ModDetector: Send + Sync + std::fmt::Debug {
    /// The mod format this detector is responsible for.
    fn mod_type(&self) -> ModType;

    /// Returns `true` if `file` is in this detector's format.
    fn matches(&self, file: &dyn ModFileAccess) -> Result<bool, ModAnalysisError>;

    /// Extract side-support and basic metadata.
    ///
    /// `matches` is guaranteed to have returned `true` before this is called.
    fn analyze(&self, file: &dyn ModFileAccess) -> Result<ModAnalysis, ModAnalysisError>;
}

/// Detects Fabric mods (and Quilt mods that ship a `fabric.mod.json`).
#[derive(Debug, Default)]
pub struct FabricDetector;

impl FabricDetector {
    pub fn new() -> Self {
        Self
    }
}

/// Subset of `fabric.mod.json` relevant for side detection.
///
/// The schema has evolved across versions; we only require the fields we use
/// and tolerate everything else via `#[serde(default)]`, so older and newer
/// manifests both deserialize without error.
#[derive(Debug, Deserialize)]
struct FabricModJson {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    name: Option<String>,
    /// Top-level environment declaration.
    ///
    /// Accepted values: `"client"`, `"server"`, `"*"` (both). When absent, the
    /// side is inferred from the `entrypoints` (see [`side_from_entrypoints`]).
    #[serde(default)]
    environment: Option<String>,
    #[serde(default)]
    entrypoints: std::collections::HashMap<String, serde_json::Value>,
}

impl ModDetector for FabricDetector {
    fn mod_type(&self) -> ModType {
        ModType::Fabric
    }

    fn matches(&self, file: &dyn ModFileAccess) -> Result<bool, ModAnalysisError> {
        Ok(file.read_entry("fabric.mod.json")?.is_some())
    }

    fn analyze(&self, file: &dyn ModFileAccess) -> Result<ModAnalysis, ModAnalysisError> {
        let raw = file
            .read_entry("fabric.mod.json")?
            .ok_or(ModAnalysisError::UnrecognizedFormat)?;
        let parsed = parse_fabric_json(&raw)?;

        let side_support = match parsed.environment.as_deref() {
            Some("client") => SideSupport::client_only(),
            Some("server") => SideSupport::server_only(),
            Some("*" | "both") => SideSupport::both(),
            // Unknown / empty / missing environment string: fall back to the
            // entrypoint-based heuristic.
            _ => side_from_entrypoints(&parsed.entrypoints),
        };

        Ok(ModAnalysis {
            mod_type: ModType::Fabric,
            side_support,
            mod_id: parsed.id,
            name: parsed.name,
            version: parsed.version,
            dependencies: Vec::new(),
        })
    }
}

/// Parse `fabric.mod.json`, tolerating the occasional raw control character
/// (e.g. an unescaped newline/tab inside a `description`) that strict JSON
/// rejects. Such characters are not meaningful for side detection, so stripping
/// them is safe and lets otherwise-valid mods parse successfully.
fn parse_fabric_json(raw: &[u8]) -> Result<FabricModJson, ModAnalysisError> {
    let text = String::from_utf8_lossy(raw);
    let cleaned: String = text.chars().filter(|c| !c.is_control()).collect();
    Ok(serde_json::from_str(&cleaned)?)
}

/// Infer side support from entrypoints.
///
/// Fabric semantics: `main` runs on both sides; `client`/`server` run only on
/// that side. A `main`-only presence is universal, so a mod is client-only only
/// when it has `client` and no `server` entrypoint, and server-only only when
/// it has `server` and no `client`. This also catches mods like Jade that omit
/// `environment` but declare a `client` entrypoint.
fn side_from_entrypoints(
    entrypoints: &std::collections::HashMap<String, serde_json::Value>,
) -> SideSupport {
    let has_client = entrypoints.contains_key("client");
    let has_server = entrypoints.contains_key("server");

    match (has_client, has_server) {
        (true, false) => SideSupport::client_only(),
        (false, true) => SideSupport::server_only(),
        // (true, true) or neither (e.g. `main`-only): usable on both sides.
        _ => SideSupport::both(),
    }
}

/// Detects Forge / NeoForge mods and derives their side support from
/// `mods.toml` (`clientSideOnly` / `serverSideOnly`) or legacy `mcmod.info`.
#[derive(Debug, Default)]
pub struct ForgeDetector;

impl ForgeDetector {
    pub fn new() -> Self {
        Self
    }
}

/// One `[[mods]]` entry inside Forge's `mods.toml`.
#[derive(Debug, Deserialize)]
struct ModsToml {
    #[serde(default)]
    mods: Vec<ModsTomlEntry>,
}

#[derive(Debug, Deserialize)]
struct ModsTomlEntry {
    #[serde(default, rename = "modId")]
    mod_id: Option<String>,
    #[serde(default, rename = "displayName")]
    display_name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    // Forge flags a mod client/server-only via these.
    #[serde(default)]
    client_side_only: Option<bool>,
    #[serde(default)]
    server_side_only: Option<bool>,
    #[serde(default, rename = "clientSideOnly")]
    client_side_only_legacy: Option<bool>,
    #[serde(default, rename = "serverSideOnly")]
    server_side_only_legacy: Option<bool>,
}

/// One entry in the legacy `mcmod.info` JSON array.
#[derive(Debug, Clone, Deserialize)]
struct McModInfo {
    #[serde(default)]
    modid: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default, rename = "clientSideOnly")]
    client_side_only: Option<bool>,
    #[serde(default, rename = "serverSideOnly")]
    server_side_only: Option<bool>,
}

impl ModDetector for ForgeDetector {
    fn mod_type(&self) -> ModType {
        ModType::Forge
    }

    fn matches(&self, file: &dyn ModFileAccess) -> Result<bool, ModAnalysisError> {
        if file.read_entry("META-INF/mods.toml")?.is_some()
            || file.read_entry("META-INF/neoforge.mods.toml")?.is_some()
            || file.read_entry("mcmod.info")?.is_some()
        {
            return Ok(true);
        }

        // Fall back to scanning the manifest for a Forge/FML marker.
        if let Some(manifest) = file.read_entry("META-INF/MANIFEST.MF")? {
            let text = String::from_utf8_lossy(&manifest);
            if text.contains("FML-System-Mods") || text.contains("ModSide") {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn analyze(&self, file: &dyn ModFileAccess) -> Result<ModAnalysis, ModAnalysisError> {
        // Prefer the modern TOML metadata (NeoForge falls back to Forge).
        let toml_raw = file
            .read_entry("META-INF/neoforge.mods.toml")?
            .or(file.read_entry("META-INF/mods.toml")?);
        if let Some(raw) = toml_raw {
            return analyze_mods_toml(file, &raw);
        }

        // Fall back to the legacy JSON `mcmod.info`.
        if let Some(raw) = file.read_entry("mcmod.info")? {
            return analyze_mcmod_info(&raw);
        }

        // Recognized as Forge via a manifest marker but no side metadata found.
        Ok(ModAnalysis {
            mod_type: ModType::Forge,
            side_support: SideSupport::unknown(),
            mod_id: None,
            name: None,
            version: None,
            dependencies: Vec::new(),
        })
    }
}

/// Extract `Implementation-Version` from a JAR manifest, used to resolve the
/// `${file.jarVersion}` placeholder in `mods.toml`.
fn manifest_impl_version(raw: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(raw);
    text.lines()
        .find_map(|line| line.trim_start().strip_prefix("Implementation-Version:"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Derive side support + identity from `mods.toml`.
///
/// A mod supports a side unless *every* declared entry restricts it away from
/// that side, so we union across entries. Forge defaults to universal when no
/// flag is set (the loader loads such mods on both sides, like `environment: "*"`).
fn analyze_mods_toml(
    file: &dyn ModFileAccess,
    raw: &[u8],
) -> Result<ModAnalysis, ModAnalysisError> {
    let text = std::str::from_utf8(raw).map_err(|_| ModAnalysisError::UnrecognizedFormat)?;
    let parsed: ModsToml = toml::from_str(text)?;

    let mut client = false;
    let mut server = false;
    let mut mod_id = None;
    let mut name = None;
    let mut version = None;
    for entry in &parsed.mods {
        let client_only = entry
            .client_side_only
            .or(entry.client_side_only_legacy)
            .unwrap_or(false);
        let server_only = entry
            .server_side_only
            .or(entry.server_side_only_legacy)
            .unwrap_or(false);
        if !server_only {
            client = true;
        }
        if !client_only {
            server = true;
        }
        if mod_id.is_none() {
            mod_id = entry.mod_id.clone();
            name = entry.display_name.clone();
            version = entry.version.clone();
        }
    }

    // Resolve the Gradle `${file.jarVersion}` placeholder from the manifest.
    if let Some(v) = version.as_ref() {
        if v.starts_with("${") {
            if let Some(manifest) = file.read_entry("META-INF/MANIFEST.MF")? {
                version = manifest_impl_version(&manifest).or(Some(v.clone()));
            }
        }
    }

    let side_support = if parsed.mods.is_empty() {
        SideSupport::unknown()
    } else {
        SideSupport { client, server }
    };

    Ok(ModAnalysis {
        mod_type: ModType::Forge,
        side_support,
        mod_id,
        name,
        version,
        dependencies: Vec::new(),
    })
}

/// Derive side support + identity from the legacy `mcmod.info` JSON array.
fn analyze_mcmod_info(raw: &[u8]) -> Result<ModAnalysis, ModAnalysisError> {
    let entries: Vec<McModInfo> = serde_json::from_slice(raw)?;
    let first = entries.first().cloned();

    let mut client = false;
    let mut server = false;
    for entry in &entries {
        let client_only = entry.client_side_only.unwrap_or(false);
        let server_only = entry.server_side_only.unwrap_or(false);
        if !server_only {
            client = true;
        }
        if !client_only {
            server = true;
        }
    }

    let side_support = if entries.is_empty() {
        SideSupport::unknown()
    } else {
        SideSupport { client, server }
    };

    let (mod_id, name, version) = first
        .map(|e| (e.modid, e.name, e.version))
        .unwrap_or((None, None, None));

    Ok(ModAnalysis {
        mod_type: ModType::Forge,
        side_support,
        mod_id,
        name,
        version,
        dependencies: Vec::new(),
    })
}

/// Orchestrates mod detection: tries each registered [`ModDetector`] against a
/// file and returns the first successful analysis.
///
/// This is the primary entry point for callers (including the server component
/// that consumes mod side-support information later).
#[derive(Debug, Default)]
pub struct ModAnalyzer {
    detectors: Vec<Box<dyn ModDetector>>,
}

impl ModAnalyzer {
    /// Create an analyzer with every known detector registered.
    ///
    /// Order matters only as a tie-breaker: the first detector whose
    /// [`ModDetector::matches`] returns `true` wins. More specific formats
    /// should therefore be registered before more generic ones.
    pub fn new() -> Self {
        Self {
            detectors: vec![
                Box::new(FabricDetector::new()),
                Box::new(ForgeDetector::new()),
            ],
        }
    }

    /// Analyze a mod located at `path` (a `.jar` file or an extracted folder).
    ///
    /// Returns [`ModAnalysis::unrecognized`] (with [`ModType::Unknown`]) when no
    /// detector recognizes the file, rather than failing — callers that only
    /// care about side support can still inspect the result safely.
    pub fn analyze_path(&self, path: impl AsRef<Path>) -> Result<ModAnalysis, ModAnalysisError> {
        let file = ModFile::open(path)?;

        for detector in &self.detectors {
            if detector.matches(&file)? {
                return detector.analyze(&file);
            }
        }

        Ok(ModAnalysis::unrecognized())
    }
}

/// Convenience helper: analyze a mod at `path` using a default [`ModAnalyzer`].
///
/// This is the simplest function to call from other crates (e.g. the server
/// component) when only a single analysis is needed.
pub fn analyze_mod_side(path: impl AsRef<Path>) -> Result<ModAnalysis, ModAnalysisError> {
    let mut analysis = ModAnalyzer::new().analyze_path(&path)?;
    // Enrich with dependency edges from the unified embedded-metadata extractor
    // so the server pruner can cascade removals through hard dependencies.
    if let Ok(bytes) = std::fs::read(path.as_ref()) {
        if let Some(meta) = extract_mod_metadata(&Bytes::from(bytes)) {
            if let Some(deps) = meta.dependencies {
                analysis.dependencies = deps
                    .into_iter()
                    .filter(|d| !is_env_dependency_id(&d.mod_id))
                    .collect();
            }
        }
    }
    Ok(analysis)
}

/// One mod file's result from [`analyze_mod_side_dir`].
pub struct ModFileAnalysis {
    /// Path of the analyzed mod file.
    pub path: PathBuf,
    /// `Ok` if analyzed; `Err` (e.g. corrupt archive) if not. The batch skips
    /// such files instead of aborting.
    pub result: Result<ModAnalysis, ModAnalysisError>,
}

/// Analyze every `.jar` directly in `dir` (non-recursive, flat `mods` layout).
///
/// Only errors if `dir` itself cannot be read; per-file failures are returned
/// in [`ModFileAnalysis::result`].
pub fn analyze_mod_side_dir(dir: impl AsRef<Path>) -> Result<Vec<ModFileAnalysis>, ModAnalysisError> {
    let dir = dir.as_ref();
    let read = std::fs::read_dir(dir).map_err(|source| ModAnalysisError::Io {
        path: dir.display().to_string(),
        source,
    })?;

    let mut out = Vec::new();
    for entry in read {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
        {
            let result = analyze_mod_side(&path);
            out.push(ModFileAnalysis { path, result });
        }
    }

    Ok(out)
}

/// Whether a mod's `environment` string marks it as client-only.
///
/// Mirrors the rule the Fabric loader enforces and that the server modpack
/// pruner relies on, so the two never drift. Returns `false` for `None`
/// (unknown / universal) as well as for `"*"` / `"server"`.
pub fn environment_is_client_only(env: Option<&str>) -> bool {
    env == Some("client")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal jar containing the given `(entry name, contents)` pairs.
    fn make_jar(path: &Path, entries: &[(&str, &str)]) {
        use std::io::Write;
        let file = std::fs::File::create(path).expect("create jar");
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::FileOptions::<()>::default();
        for (name, contents) in entries {
            zip.start_file(*name, opts).expect("start file");
            zip.write_all(contents.as_bytes()).expect("write entry");
        }
        zip.finish().expect("finish jar");
    }

    #[test]
    fn fabric_environment_client_only() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("mod.jar");
        make_jar(&jar, &[("fabric.mod.json", r#"{"environment":"client","id":"x"}"#)]);
        let a = analyze_mod_side(&jar).unwrap();
        assert_eq!(a.mod_type, ModType::Fabric);
        assert!(a.side_support.is_client_only());
    }

    #[test]
    fn fabric_environment_server_only() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("mod.jar");
        make_jar(&jar, &[("fabric.mod.json", r#"{"environment":"server","id":"x"}"#)]);
        let a = analyze_mod_side(&jar).unwrap();
        assert!(a.side_support.is_server_only());
    }

    #[test]
    fn fabric_environment_both() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("mod.jar");
        make_jar(&jar, &[("fabric.mod.json", r#"{"environment":"*","id":"x"}"#)]);
        let a = analyze_mod_side(&jar).unwrap();
        assert!(a.side_support.is_universal());
    }

    /// Jade ships a `client` entrypoint plus a `main` entrypoint but no `server`
    /// entrypoint. It must be identified as a client-only mod even though `main`
    /// loads on both sides.
    #[test]
    fn fabric_client_only_inferred_from_entrypoints_like_jade() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("jade.jar");
        make_jar(
            &jar,
            &[(
                "fabric.mod.json",
                r#"{"id":"jade","entrypoints":{"main":["snownee.jade.util.CommonProxy"],"client":["snownee.jade.util.ClientProxy"]}}"#,
            )],
        );
        let a = analyze_mod_side(&jar).unwrap();
        assert!(a.side_support.is_client_only());
    }

    #[test]
    fn fabric_universal_when_only_main_entrypoint() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("lib.jar");
        make_jar(
            &jar,
            &[("fabric.mod.json", r#"{"entrypoints":{"main":["com.example.Common"]}}"#)],
        );
        let a = analyze_mod_side(&jar).unwrap();
        assert!(a.side_support.is_universal());
    }

    #[test]
    fn forge_client_side_only_from_mods_toml() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("forge-mod.jar");
        let toml = "[[mods]]\nmodId = \"example\"\nclientSideOnly = true\nserverSideOnly = false\n";
        make_jar(&jar, &[("META-INF/mods.toml", toml)]);
        let a = analyze_mod_side(&jar).unwrap();
        assert_eq!(a.mod_type, ModType::Forge);
        assert!(a.side_support.is_client_only());
    }

#[test]
fn forge_universal_from_mods_toml() {
    let dir = tempfile::tempdir().unwrap();
    let jar = dir.path().join("mod.jar");
    let toml = "modLoader = \"javafml\"\nloaderVersion = \"[43,)\"\n\n[[mods]]\nmodId = \"myforge\"\nclientSideOnly = false\nserverSideOnly = false\n";
    make_jar(&jar, &[("META-INF/mods.toml", toml)]);

    let analysis = analyze_mod_side(&jar).unwrap();
    assert_eq!(analysis.mod_type, ModType::Forge);
    assert!(analysis.side_support.is_universal());
}

#[test]
fn forge_mods_toml_extracts_identity_and_defaults_universal() {
    let dir = tempfile::tempdir().unwrap();
    let jar = dir.path().join("sc.jar");
    let toml = "modLoader = \"javafml\"\nloaderVersion = \"[43,)\"\n\n[[mods]]\nmodId = \"securitycraft\"\ndisplayName = \"SecurityCraft\"\nversion = \"${file.jarVersion}\"\n";
    let manifest = "Manifest-Version: 1.0\nImplementation-Version: 1.9.6.1\n";
    make_jar(
        &jar,
        &[
            ("META-INF/mods.toml", toml),
            ("META-INF/MANIFEST.MF", manifest),
        ],
    );

    let analysis = analyze_mod_side(&jar).unwrap();
    assert_eq!(analysis.mod_type, ModType::Forge);
    assert_eq!(analysis.mod_id.as_deref(), Some("securitycraft"));
    assert_eq!(analysis.name.as_deref(), Some("SecurityCraft"));
    assert_eq!(analysis.version.as_deref(), Some("1.9.6.1"));
    assert!(analysis.side_support.is_universal());
}

    #[test]
    fn unrecognized_file_is_unknown_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("unknown.jar");
        make_jar(&jar, &[("random.txt", "hello")]);
        let a = analyze_mod_side(&jar).unwrap();
        assert_eq!(a.mod_type, ModType::Unknown);
    }

    #[test]
    fn extracted_directory_is_analyzed_like_jar() {
        let dir = tempfile::tempdir().unwrap();
        let mod_dir = dir.path().join("extracted-mod");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(mod_dir.join("fabric.mod.json"), r#"{"environment":"client","id":"x"}"#).unwrap();
        let a = analyze_mod_side(&mod_dir).unwrap();
        assert!(a.side_support.is_client_only());
    }

    #[test]
    fn analyze_dir_returns_one_entry_per_jar() {
        let dir = tempfile::tempdir().unwrap();
        make_jar(
            &dir.path().join("a.jar"),
            &[("fabric.mod.json", r#"{"environment":"client","id":"a"}"#)],
        );
        make_jar(
            &dir.path().join("b.jar"),
            &[("fabric.mod.json", r#"{"environment":"*","id":"b"}"#)],
        );
        // Non-jar files must be ignored.
        std::fs::write(dir.path().join("notes.txt"), "not a mod").unwrap();

        let results = analyze_mod_side_dir(dir.path()).unwrap();
        assert_eq!(results.len(), 2, "only the two jars should be analyzed");

        let client = results
            .iter()
            .find(|r| r.path.file_name().unwrap() == "a.jar")
            .expect("a.jar present");
        assert!(client.result.as_ref().unwrap().side_support.is_client_only());

        let universal = results
            .iter()
            .find(|r| r.path.file_name().unwrap() == "b.jar")
            .expect("b.jar present");
        assert!(universal.result.as_ref().unwrap().side_support.is_universal());
    }

    #[test]
    fn analyze_dir_keeps_going_past_broken_jar() {
        let dir = tempfile::tempdir().unwrap();
        make_jar(
            &dir.path().join("good.jar"),
            &[("fabric.mod.json", r#"{"environment":"*","id":"g"}"#)],
        );
        // A corrupt archive is reported as an error for that file, not fatal.
        std::fs::write(dir.path().join("bad.jar"), b"not a zip").unwrap();

        let results = analyze_mod_side_dir(dir.path()).unwrap();
        assert_eq!(results.len(), 2);

        let bad = results
            .iter()
            .find(|r| r.path.file_name().unwrap() == "bad.jar")
            .expect("bad.jar present");
        assert!(bad.result.is_err());
    }

    #[test]
    fn environment_is_client_only_rule() {
        assert!(environment_is_client_only(Some("client")));
        assert!(!environment_is_client_only(Some("*")));
        assert!(!environment_is_client_only(Some("server")));
        assert!(!environment_is_client_only(None));
    }

    /// Run against a real mod jar supplied via `MOD_ANALYZER_TEST_JAR`.
    #[test]
    #[ignore]
    fn analyze_real_jar_from_env() {
        let path = std::env::var("MOD_ANALYZER_TEST_JAR").expect("set MOD_ANALYZER_TEST_JAR");
        let a = analyze_mod_side(&path).expect("analyze");
        println!("mod_type: {:#?}", a.mod_type);
        println!("side_support: {:#?}", a.side_support);
        println!("supports_server: {}", a.supports_server());
    }
}
