//! Shared helpers for extracting content from local modpack archives.

use std::path::{Component, Path, PathBuf};

use super::detect::decode_zip_entry_name;
use crate::util::io;

const EXTRACTION_SIZE_LIMIT: u64 = 8 * 1024 * 1024 * 1024;

fn archive_error(error: zip::result::ZipError) -> crate::Error {
    crate::ErrorKind::InputError(format!(
        "Modpack archive is invalid: {error}"
    ))
    .into()
}

pub(crate) fn safe_relative_path(value: &str) -> crate::Result<String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(crate::ErrorKind::InputError(
            "Modpack archive contains an invalid file path".to_string(),
        )
        .into());
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
}

/// Extracts every file under `prefix` in the archive into `target_dir`,
/// preserving the directory structure below the prefix. Returns the number of
/// files written.
pub(crate) async fn extract_archive_subdir(
    archive_path: PathBuf,
    prefix: String,
    target_dir: PathBuf,
) -> crate::Result<u32> {
    tokio::task::spawn_blocking(move || {
        extract_archive_subdir_sync(&archive_path, &prefix, &target_dir)
    })
    .await?
}

fn extract_archive_subdir_sync(
    archive_path: &Path,
    prefix: &str,
    target_dir: &Path,
) -> crate::Result<u32> {
    let file = std::fs::File::open(archive_path)
        .map_err(|error| io::IOError::with_path(error, archive_path))?;
    let mut archive = zip::ZipArchive::new(file).map_err(archive_error)?;
    let mut files_written = 0_u32;
    let mut total_size = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        let entry_name = decode_zip_entry_name(entry.name_raw());
        if entry.is_dir() || !entry_name.starts_with(prefix) {
            continue;
        }
        let relative = &entry_name[prefix.len()..];
        if relative.is_empty() {
            continue;
        }
        total_size = total_size.saturating_add(entry.size());
        if total_size > EXTRACTION_SIZE_LIMIT {
            return Err(crate::ErrorKind::InputError(
                "Modpack archive contents exceed the extraction limit"
                    .to_string(),
            )
            .into());
        }
        let target = target_dir.join(safe_relative_path(relative)?);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| io::IOError::with_path(error, parent))?;
        }
        let mut output = std::fs::File::create(&target)
            .map_err(|error| io::IOError::with_path(error, &target))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| io::IOError::with_path(error, &target))?;
        files_written = files_written.saturating_add(1);
    }
    Ok(files_written)
}

/// Extracts a single archive entry to the given target file path.
pub(crate) async fn extract_archive_entry_to_file(
    archive_path: PathBuf,
    entry_name: String,
    target: PathBuf,
) -> crate::Result<()> {
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive_path)
            .map_err(|error| io::IOError::with_path(error, &archive_path))?;
        let mut archive = zip::ZipArchive::new(file).map_err(archive_error)?;
        let index = (0..archive.len())
            .find(|&index| {
                archive
                    .by_index_raw(index)
                    .map(|entry| {
                        decode_zip_entry_name(entry.name_raw()) == entry_name
                    })
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Modpack archive is missing {entry_name}"
                ))
            })?;
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| io::IOError::with_path(error, parent))?;
        }
        let mut output = std::fs::File::create(&target)
            .map_err(|error| io::IOError::with_path(error, &target))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| io::IOError::with_path(error, &target))?;
        Ok(())
    })
    .await?
}

/// Reads a single archive entry into a string, tolerating GB18030-encoded
/// file contents produced by Chinese packaging tools.
pub(crate) async fn read_archive_entry_to_string(
    archive_path: PathBuf,
    entry_name: String,
) -> crate::Result<String> {
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive_path)
            .map_err(|error| io::IOError::with_path(error, &archive_path))?;
        let mut archive = zip::ZipArchive::new(file).map_err(archive_error)?;
        let index =
            super::detect::find_entry_index(&mut archive, &entry_name)?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(format!(
                        "Modpack archive is missing {entry_name}"
                    ))
                })?;
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        let mut contents = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut contents)?;
        Ok(match String::from_utf8(contents) {
            Ok(value) => value,
            Err(error) => {
                let (decoded, _, _) =
                    encoding_rs::GB18030.decode(error.as_bytes());
                decoded.into_owned()
            }
        })
    })
    .await?
}

/// Allocates a unique scratch directory for extracting nested pack content.
pub(crate) async fn create_import_scratch_dir(
    state: &crate::State,
) -> crate::Result<PathBuf> {
    let dir = state
        .directories
        .caches_dir()
        .join("modpack-import")
        .join(uuid::Uuid::new_v4().to_string());
    io::create_dir_all(&dir).await?;
    Ok(dir)
}
