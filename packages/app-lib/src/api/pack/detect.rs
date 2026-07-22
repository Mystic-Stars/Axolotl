//! Local modpack file format detection.
//!
//! Detects the modpack format of a local archive by inspecting its contents
//! rather than its file extension, checking both the archive root and a single
//! wrapping folder, mirroring the detection behavior of PCL.

use std::io::Read;
use std::path::Path;

pub const MRPACK_MANIFEST: &str = "modrinth.index.json";
pub const CURSEFORGE_MANIFEST: &str = "manifest.json";
pub const MCBBS_MANIFEST: &str = "mcbbs.packmeta";
pub const HMCL_MANIFEST: &str = "modpack.json";
pub const MMC_MANIFEST: &str = "mmc-pack.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPackFormat {
    Mrpack,
    CurseForge,
    Mcbbs,
    Hmcl,
    MmcExport,
    LauncherBundled,
    PlainArchive,
}

impl LocalPackFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mrpack => "Modrinth",
            Self::CurseForge => "CurseForge",
            Self::Mcbbs => "MCBBS",
            Self::Hmcl => "HMCL",
            Self::MmcExport => "MultiMC",
            Self::LauncherBundled => "launcher bundle",
            Self::PlainArchive => "game folder archive",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DetectedLocalPack {
    pub format: LocalPackFormat,
    /// Prefix of the folder containing the pack's key files, either empty or
    /// a single path segment ending in `/`.
    pub base_folder: String,
    /// For [`LocalPackFormat::LauncherBundled`], the archive entry of the
    /// nested modpack file.
    pub inner_pack_entry: Option<String>,
    /// For [`LocalPackFormat::PlainArchive`], the version id matched under
    /// `versions/<id>/<id>.json`.
    pub plain_version_id: Option<String>,
}

/// Decodes a zip entry name, tolerating archives produced by Chinese tools
/// that store GB18030-encoded names without the UTF-8 flag.
pub fn decode_zip_entry_name(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(name) => name.to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GB18030.decode(raw);
            decoded.into_owned()
        }
    }
}

pub async fn detect_local_pack(
    path: &Path,
) -> crate::Result<DetectedLocalPack> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || detect_local_pack_sync(&path)).await?
}

fn open_error(path: &Path, error: impl std::fmt::Display) -> crate::Error {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rar"))
    {
        crate::ErrorKind::InputError(
            "RAR modpack archives are not supported; please repackage the modpack as a zip file".to_string(),
        )
        .into()
    } else {
        crate::ErrorKind::InputError(format!(
            "Failed to open modpack archive: {error}"
        ))
        .into()
    }
}

fn detect_local_pack_sync(path: &Path) -> crate::Result<DetectedLocalPack> {
    let file =
        std::fs::File::open(path).map_err(|error| open_error(path, error))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| open_error(path, error))?;

    let mut names = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Failed to read modpack archive entry: {error}"
            ))
        })?;
        if entry.encrypted() {
            return Err(crate::ErrorKind::InputError(
                "Encrypted modpack archives are not supported".to_string(),
            )
            .into());
        }
        names.push(decode_zip_entry_name(entry.name_raw()));
    }

    let bases = candidate_bases(&names);
    for base in &bases {
        if let Some(detected) = detect_at_base(&mut archive, &names, base)? {
            return Ok(detected);
        }
    }

    if let Some(detected) = detect_plain_archive(&names) {
        return Ok(detected);
    }

    Err(crate::ErrorKind::InputError(
        "Unrecognized modpack format: no known pack manifest was found in the archive"
            .to_string(),
    )
    .into())
}

/// The archive root, followed by each distinct single wrapping folder.
fn candidate_bases(names: &[String]) -> Vec<String> {
    let mut bases = vec![String::new()];
    for name in names {
        if let Some((first, rest)) = name.split_once('/')
            && !rest.is_empty()
            && !rest.contains('/')
        {
            let base = format!("{first}/");
            if !bases.contains(&base) {
                bases.push(base);
            }
        }
    }
    bases
}

fn detect_at_base<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    names: &[String],
    base: &str,
) -> crate::Result<Option<DetectedLocalPack>> {
    let has = |file: &str| -> bool {
        let target = format!("{base}{file}");
        names.iter().any(|name| name == &target)
    };
    let detected = |format: LocalPackFormat| DetectedLocalPack {
        format,
        base_folder: base.to_string(),
        inner_pack_entry: None,
        plain_version_id: None,
    };

    // MCBBS and MultiMC packs may also contain a manifest.json, so both must
    // be checked before the CurseForge manifest.
    if has(MCBBS_MANIFEST) {
        return Ok(Some(detected(LocalPackFormat::Mcbbs)));
    }
    if has(MMC_MANIFEST) {
        return Ok(Some(detected(LocalPackFormat::MmcExport)));
    }
    if has(MRPACK_MANIFEST) {
        return Ok(Some(detected(LocalPackFormat::Mrpack)));
    }
    if has(CURSEFORGE_MANIFEST) {
        // A manifest.json with an `addons` array is the MCBBS variant. An
        // unreadable manifest.json (e.g. an unrelated mod config inside a
        // zipped game folder) does not abort detection of other formats.
        match read_entry_json(archive, &format!("{base}{CURSEFORGE_MANIFEST}"))
        {
            Ok(manifest) => {
                return Ok(Some(detected(
                    if manifest
                        .get("addons")
                        .is_some_and(|value| !value.is_null())
                    {
                        LocalPackFormat::Mcbbs
                    } else {
                        LocalPackFormat::CurseForge
                    },
                )));
            }
            Err(error) => {
                tracing::warn!(
                    "Ignoring unparsable manifest.json at {base:?} during modpack detection: {error}"
                );
            }
        }
    }
    if has(HMCL_MANIFEST) {
        return Ok(Some(detected(LocalPackFormat::Hmcl)));
    }
    for inner in ["modpack.zip", "modpack.mrpack"] {
        if has(inner) {
            return Ok(Some(DetectedLocalPack {
                format: LocalPackFormat::LauncherBundled,
                base_folder: base.to_string(),
                inner_pack_entry: Some(format!("{base}{inner}")),
                plain_version_id: None,
            }));
        }
    }

    Ok(None)
}

/// Finds an entry index by its decoded name, so lookups stay consistent with
/// [`decode_zip_entry_name`] even for archives with GB18030-encoded names.
pub(crate) fn find_entry_index<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    entry_name: &str,
) -> crate::Result<Option<usize>> {
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Failed to read modpack archive entry: {error}"
            ))
        })?;
        if decode_zip_entry_name(entry.name_raw()) == entry_name {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn read_entry_json<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    entry_name: &str,
) -> crate::Result<serde_json::Value> {
    let index = find_entry_index(archive, entry_name)?.ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Modpack archive is missing {entry_name}"
        ))
    })?;
    let mut entry = archive.by_index(index).map_err(|error| {
        crate::ErrorKind::InputError(format!(
            "Failed to read {entry_name} from modpack archive: {error}"
        ))
    })?;
    let mut contents = Vec::new();
    entry.read_to_end(&mut contents)?;
    Ok(serde_json::from_slice(&contents)?)
}

/// Looks for a `versions/<id>/<id>.json` structure marking a zipped-up game
/// folder, returning the prefix of the folder containing `versions`.
fn detect_plain_archive(names: &[String]) -> Option<DetectedLocalPack> {
    for name in names {
        let segments: Vec<&str> = name.split('/').collect();
        if segments.len() < 3 {
            continue;
        }
        let json = segments[segments.len() - 1];
        let version = segments[segments.len() - 2];
        let marker = segments[segments.len() - 3];
        if marker == "versions"
            && !version.is_empty()
            && json
                .strip_suffix(".json")
                .is_some_and(|stem| stem == version)
        {
            let base = segments[..segments.len() - 3].join("/");
            let base = if base.is_empty() {
                base
            } else {
                format!("{base}/")
            };
            return Some(DetectedLocalPack {
                format: LocalPackFormat::PlainArchive,
                base_folder: base,
                inner_pack_entry: None,
                plain_version_id: Some(version.to_string()),
            });
        }
    }
    None
}
