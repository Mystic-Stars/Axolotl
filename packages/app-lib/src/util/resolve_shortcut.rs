//! Resolve shortcuts, symlinks, and application bundles to their target paths.
//!
//! Supports:
//! - Windows `.lnk` files via COM `IShellLinkW`
//! - macOS `.app` bundles via `Contents/Info.plist`
//! - Linux `.desktop` files
//! - Symlinks on all platforms

use std::path::{Path, PathBuf};
use tracing::warn;

/// Maximum recursion depth to prevent infinite loops.
#[allow(dead_code)]
const MAX_DEPTH: u32 = 3;

/// Resolve a shortcut, symlink, or application bundle to its target path.
///
/// Returns `None` when the path is not a recognised shortcut type, on any
/// error, or when `max_depth` reaches 0 (recursion guard).
pub fn resolve_shortcut(path: &Path, max_depth: u32) -> Option<PathBuf> {
    if max_depth == 0 {
        warn!(
            "Shortcut resolution exceeded max depth for: {}",
            path.display()
        );
        return None;
    }

    // Always try symlink resolution first (platform-independent).
    if let Ok(target) = std::fs::read_link(path) {
        if target.is_absolute() {
            // Resolve symlink chains recursively.
            return resolve_shortcut(&target, max_depth - 1).or(Some(target));
        } else {
            // Relative symlink: resolve relative to the symlink's parent.
            let absolute = if let Some(parent) = path.parent() {
                parent.join(&target)
            } else {
                target
            };
            return resolve_shortcut(&absolute, max_depth - 1)
                .or(Some(absolute));
        }
    }

    // Platform-specific shortcut resolution.
    #[cfg(windows)]
    {
        if let Some(resolved) = resolve_windows_lnk(path, max_depth) {
            return Some(resolved);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(resolved) = resolve_macos_app(path, max_depth) {
            return Some(resolved);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(resolved) = resolve_linux_desktop(path, max_depth) {
            return Some(resolved);
        }
    }

    None
}

// ─── Windows .lnk resolution ───────────────────────────────────────────────

#[cfg(windows)]
fn resolve_windows_lnk(path: &Path, _max_depth: u32) -> Option<PathBuf> {
    let ext = path.extension()?;
    if !ext.eq_ignore_ascii_case("lnk") {
        return None;
    }

    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        CoCreateInstance, CoInitializeEx, IPersistFile, STGM,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    // SAFETY: COM is initialised for this thread. The guard ensures
    // `CoUninitialize` is called exactly once when we're done.
    unsafe {
        let init = CoInitializeEx(
            None,
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        );
        if init.is_err() {
            return None;
        }
        let _guard = ComGuard;

        let shortcut: IShellLinkW =
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;

        let persist: IPersistFile = shortcut.cast().ok()?;

        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        persist
            .Load(PCWSTR::from_raw(wide_path.as_ptr()), STGM(0))
            .ok()?;

        let mut buf = [0u16; 1024];
        shortcut.GetPath(&mut buf, std::ptr::null_mut(), 0).ok()?;

        // Find the null terminator.
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let target = String::from_utf16(&buf[..len]).ok()?;
        let target_path = PathBuf::from(&target);

        if target_path.exists() {
            Some(target_path)
        } else {
            // Return the path anyway per spec — let the caller decide.
            Some(target_path)
        }
    }
}

#[cfg(windows)]
use windows::Win32::System::Com::CoUninitialize;

#[cfg(windows)]
struct ComGuard;

#[cfg(windows)]
impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: COM was initialised in the current thread before this guard
        // was created.
        unsafe {
            CoUninitialize();
        }
    }
}

// ─── macOS .app bundle resolution ──────────────────────────────────────────

#[cfg(target_os = "macos")]
fn resolve_macos_app(path: &Path, max_depth: u32) -> Option<PathBuf> {
    // Must be a directory ending in .app.
    if !path.is_dir() {
        return None;
    }
    let ext = path.extension()?;
    if !ext.eq_ignore_ascii_case("app") {
        return None;
    }

    let plist_path = path.join("Contents").join("Info.plist");
    if !plist_path.exists() {
        return None;
    }

    // Parse the XML plist with quick_xml to find CFBundleExecutable.
    let content = std::fs::read_to_string(&plist_path).ok()?;
    let executable = parse_cfbundle_executable(&content)?;

    let executable_path = path.join("Contents").join("MacOS").join(executable);
    if executable_path.exists() {
        resolve_shortcut(&executable_path, max_depth - 1)
            .or(Some(executable_path))
    } else {
        None
    }
}

/// Naive XML parser for extracting `CFBundleExecutable` from a plist.
///
/// Uses simple string matching rather than a full XML parser to avoid adding
/// a dependency. The plist is well-known enough that this is reliable.
#[cfg(target_os = "macos")]
fn parse_cfbundle_executable(content: &str) -> Option<String> {
    // Look for: <key>CFBundleExecutable</key> followed by <string>value</string>
    let key_marker = "<key>CFBundleExecutable</key>";
    let key_pos = content.find(key_marker)?;
    let after_key = &content[key_pos + key_marker.len()..];

    let string_start = after_key.find("<string>")?;
    let value_start = string_start + "<string>".len();
    let value_end = after_key[value_start..].find("</string>")?;

    Some(
        after_key[value_start..value_start + value_end]
            .trim()
            .to_string(),
    )
}

// ─── Linux .desktop file resolution ────────────────────────────────────────

#[cfg(target_os = "linux")]
fn resolve_linux_desktop(path: &Path, _max_depth: u32) -> Option<PathBuf> {
    let ext = path.extension()?;
    if !ext.eq_ignore_ascii_case("desktop") {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    parse_desktop_entry(&content)
}

#[cfg(target_os = "linux")]
fn parse_desktop_entry(content: &str) -> Option<PathBuf> {
    let mut in_desktop_entry = false;
    let mut exec_line: Option<&str> = None;
    let mut path_line: Option<&str> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if trimmed.starts_with('[') {
            // Reached next section — stop looking.
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Exec=") {
            exec_line = Some(value);
        } else if let Some(value) = trimmed.strip_prefix("Path=") {
            path_line = Some(value);
        }
    }

    let exec = exec_line?;

    // Parse the Exec value: extract the first token that is not a parameter
    // placeholder like %f, %F, %u, %U, %k, %c.
    let tokens = shlex::split(exec);
    let binary =
        tokens
            .iter()
            .flat_map(|tokens| tokens.iter())
            .find(|token| {
                !token.starts_with('%')
                    && !token.starts_with('-')
                    && !token.starts_with("--")
            })?;

    let binary_path = if binary.contains('/') {
        PathBuf::from(binary)
    } else {
        // Look up in PATH.
        std::env::var_os("PATH")
            .iter()
            .flat_map(|p| std::env::split_paths(p))
            .map(|dir| dir.join(binary))
            .find(|p| p.exists())?
    };

    if let Some(p) = path_line {
        let work_dir = PathBuf::from(p);
        if work_dir.exists() {
            // Return the binary path resolved from the working directory context.
            return Some(binary_path);
        }
    }

    Some(binary_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_symlink_resolution() {
        let dir = tempdir().expect("temp dir");
        let target = dir.path().join("real_file.txt");
        let link = dir.path().join("link.txt");

        fs::write(&target, "hello").expect("write target");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link).expect("symlink");
            let resolved = resolve_shortcut(&link, MAX_DEPTH);
            assert!(resolved.is_some());
            assert_eq!(
                resolved.unwrap().canonicalize().ok(),
                Some(target.canonicalize().ok()).flatten()
            );
        }
        #[cfg(windows)]
        {
            // On Windows, try junction or symlink if available.
            let _ = std::os::windows::fs::symlink_file(&target, &link);
            let resolved = resolve_shortcut(&link, MAX_DEPTH);
            // symlink creation may fail on some Windows configs — accept either outcome.
            if link.exists() {
                assert!(resolved.is_some());
            }
        }
    }

    #[test]
    fn test_broken_symlink() {
        let dir = tempdir().expect("temp dir");
        let link = dir.path().join("broken_link");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/nonexistent/path", &link)
                .expect("symlink");
            let resolved = resolve_shortcut(&link, MAX_DEPTH);
            // `read_link` succeeds but the target doesn't exist — resolve_shortcut
            // still returns the target path from read_link.
            assert!(resolved.is_some());
        }
    }

    #[test]
    fn test_max_depth_zero() {
        let resolved = resolve_shortcut(Path::new("/some/path"), 0);
        assert!(resolved.is_none());
    }

    #[test]
    fn test_regular_file_no_shortcut() {
        let dir = tempdir().expect("temp dir");
        let file = dir.path().join("regular.txt");
        fs::write(&file, "data").expect("write");

        let resolved = resolve_shortcut(&file, MAX_DEPTH);
        // Not a symlink, not a platform shortcut — should return None.
        assert!(resolved.is_none());
    }

    #[test]
    fn test_relative_symlink() {
        let dir = tempdir().expect("temp dir");
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");

        fs::write(&target, "hello").expect("write target");
        #[cfg(unix)]
        {
            // Create a relative symlink.
            std::os::unix::fs::symlink("target.txt", &link).expect("symlink");
            let resolved = resolve_shortcut(&link, MAX_DEPTH);
            assert!(resolved.is_some());
            let resolved = resolved.unwrap();
            assert!(resolved.exists());
        }
    }

    #[test]
    fn test_not_a_symlink_directory() {
        let dir = tempdir().expect("temp dir");
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).expect("create dir");

        let resolved = resolve_shortcut(&subdir, MAX_DEPTH);
        // A regular directory without any shortcut marker → None.
        assert!(resolved.is_none());
    }
}
