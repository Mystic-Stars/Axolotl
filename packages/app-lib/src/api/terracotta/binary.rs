use eyre::{Context, bail};
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn terracotta_platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-arm64",
        ("linux", "riscv64") => "linux-riscv64",
        ("linux", "loongarch64") => "linux-loongarch64",
        ("macos", "x86_64") => "macos-x86_64",
        ("macos", "aarch64") => "macos-arm64",
        ("windows", "x86_64") => "windows-x86_64",
        ("windows", "aarch64") => "windows-arm64",
        ("freebsd", "x86_64") => "freebsd-x86_64",
        _ => "unsupported",
    }
}

pub(super) fn terracotta_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "terracotta.exe"
    } else {
        "terracotta"
    }
}

pub(super) fn versioned_terracotta_binary_name(
    version: &str,
    platform: &str,
) -> String {
    let extension = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    format!("terracotta-{version}-{platform}{extension}")
}

pub(super) fn validate_terracotta_version(version: &str) -> eyre::Result<()> {
    if version.is_empty()
        || version.len() > 64
        || version.contains("..")
        || !version.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '-' | '_')
        })
    {
        bail!("invalid terracotta version: {version}");
    }
    Ok(())
}

pub(super) fn is_terracotta_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.file_type().is_file() {
        return false;
    }

    let mut magic = [0_u8; 4];
    if std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut magic))
        .is_err()
    {
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        matches!(
            magic,
            [0xce, 0xfa, 0xed, 0xfe]
                | [0xcf, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xce]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
                | [0xca, 0xfe, 0xba, 0xbf]
                | [0xbf, 0xba, 0xfe, 0xca]
        )
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        magic == [0x7f, b'E', b'L', b'F']
    }

    #[cfg(target_os = "windows")]
    {
        magic[0..2] == [b'M', b'Z']
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "windows"
    )))]
    {
        false
    }
}

pub(super) fn find_terracotta_executable(
    dir: &Path,
    preferred_name: Option<&str>,
) -> Option<PathBuf> {
    let mut pending = vec![dir.to_path_buf()];
    let mut candidates = Vec::new();

    while let Some(current) = pending.pop() {
        let entries = match std::fs::read_dir(current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }

            let Some(name) = path.file_name().and_then(|name| name.to_str())
            else {
                continue;
            };
            if (name == terracotta_binary_name()
                || name.starts_with("terracotta-"))
                && is_terracotta_executable(&path)
            {
                candidates.push(path);
            }
        }
    }

    candidates.sort();
    preferred_name
        .and_then(|preferred| {
            candidates
                .iter()
                .find(|path| {
                    path.file_name().and_then(|name| name.to_str())
                        == Some(preferred)
                })
                .cloned()
        })
        .or_else(|| candidates.into_iter().next())
}

pub(super) fn terracotta_binary_path() -> PathBuf {
    let base_dir = crate::state::DirectoryInfo::global_handle_if_ready()
        .map(|directories| directories.config_dir.clone())
        .or_else(|| {
            crate::state::DirectoryInfo::initial_settings_dir_path(
                crate::brand::BUNDLE_IDENTIFIER,
            )
        })
        .unwrap_or_else(|| PathBuf::from("."));

    terracotta_binary_path_in(&base_dir)
}

pub(super) fn terracotta_binary_path_in(base_dir: &Path) -> PathBuf {
    base_dir.join("terracotta").join(terracotta_binary_name())
}

fn legacy_terracotta_binary_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("terracotta")
        .join(terracotta_binary_name())
}

pub fn terracotta_download_urls(version: &str, platform: &str) -> Vec<String> {
    let artifact = format!("terracotta-{version}-{platform}-pkg.tar.gz");
    vec![
        format!(
            "https://gitee.com/burningtnt/Terracotta/releases/download/v{version}/{artifact}"
        ),
        format!(
            "https://github.com/burningtnt/Terracotta/releases/download/v{version}/{artifact}"
        ),
    ]
}

pub(super) fn resolve_terracotta_binary_path(bin_path: &Path) -> PathBuf {
    let resolved_path = if bin_path.is_absolute() {
        bin_path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(bin_path)
    };

    let preferred_path = if resolved_path.is_dir() {
        resolved_path.join(terracotta_binary_name())
    } else if resolved_path.is_file() {
        resolved_path
    } else {
        resolved_path.with_file_name(terracotta_binary_name())
    };

    if is_terracotta_executable(&preferred_path) {
        return preferred_path;
    }

    preferred_path
        .parent()
        .and_then(|parent| find_terracotta_executable(parent, None))
        .unwrap_or(preferred_path)
}

pub(super) fn resolve_installed_terracotta_binary_path() -> PathBuf {
    resolve_installed_terracotta_binary_path_from(
        &terracotta_binary_path(),
        &legacy_terracotta_binary_path(),
    )
}

pub(super) fn resolve_installed_terracotta_binary_path_from(
    installed_path: &Path,
    legacy_path: &Path,
) -> PathBuf {
    let installed_path = resolve_terracotta_binary_path(installed_path);
    if is_terracotta_executable(&installed_path) {
        return installed_path;
    }

    let legacy_path = resolve_terracotta_binary_path(legacy_path);
    if is_terracotta_executable(&legacy_path) {
        legacy_path
    } else {
        installed_path
    }
}

pub(super) async fn install_terracotta_binary(
    candidate: &Path,
    destination: &Path,
) -> eyre::Result<()> {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre::eyre!("invalid terracotta destination path"))?;
    let backup = destination.with_file_name(format!("{file_name}.old"));

    if backup.exists() {
        tokio::fs::remove_file(&backup)
            .await
            .wrap_err("failed to remove stale terracotta backup")?;
    }
    if destination.exists() {
        tokio::fs::rename(destination, &backup)
            .await
            .wrap_err("failed to back up the existing terracotta binary")?;
    }

    if let Err(error) = tokio::fs::rename(candidate, destination).await {
        if backup.exists() {
            let _ = tokio::fs::rename(&backup, destination).await;
        }
        return Err(error).wrap_err("failed to install terracotta binary");
    }

    if backup.exists() {
        tokio::fs::remove_file(&backup).await.wrap_err(
            "failed to remove terracotta backup after installation",
        )?;
    }
    Ok(())
}

pub(super) async fn cleanup_legacy_versions(
    new_version: &str,
) -> eyre::Result<()> {
    let target_dir = terracotta_binary_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("terracotta"));

    if !target_dir.exists() {
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(&target_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry.file_type().await?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if file_type.is_file()
            && (should_cleanup_terracotta_file(&name)
                || name.ends_with(".tar.gz")
                || name.ends_with(".old"))
        {
            tokio::fs::remove_file(entry.path()).await?;
            info!(
                "removed extracted terracotta artifact after installing {new_version}: {name}"
            );
        }

        if file_type.is_dir() && name.starts_with("terracotta-") {
            tokio::fs::remove_dir_all(entry.path()).await?;
            info!(
                "removed extracted terracotta directory after installing {new_version}: {name}"
            );
        }
    }

    Ok(())
}

pub(super) fn should_cleanup_terracotta_file(name: &str) -> bool {
    name.starts_with("terracotta-")
        && name != "terracotta-version.json"
        && name != "terracotta-version.json.tmp"
}
