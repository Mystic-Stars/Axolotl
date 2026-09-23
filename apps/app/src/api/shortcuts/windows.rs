use crate::api::Result;
use std::{
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use url::Url;
use windows::{
    Win32::{
        Storage::EnhancedStorage::PKEY_AppUserModel_ID,
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx,
            CoUninitialize, IPersistFile, StructuredStorage::PROPVARIANT,
        },
        UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink},
    },
    core::{Interface, PCWSTR},
};

pub(super) const SHORTCUT_EXTENSION: &str = "lnk";
pub(super) const SHORTCUT_ICON_EXTENSION: &str = "ico";

pub(super) async fn create_shortcut(
    _profile_name: &str,
    launch_url: &Url,
    output_path: &Path,
    icon_path: Option<&Path>,
    app_identifier: &str,
) -> Result<()> {
    let target_path = std::env::current_exe()?;
    let working_dir = target_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let output_path = output_path.to_path_buf();
    let icon_path = icon_path.map(Path::to_path_buf);
    let launch_url = launch_url.to_string();
    let app_identifier = app_identifier.to_string();

    tokio::task::spawn_blocking(move || {
        create_windows_shortcut(
            output_path,
            target_path,
            working_dir,
            launch_url,
            icon_path,
            app_identifier,
        )
    })
    .await
    .map_err(|error| {
        std::io::Error::other(format!(
            "failed to join shortcut creation task: {error}"
        ))
    })??;

    Ok(())
}

fn create_windows_shortcut(
    output_path: PathBuf,
    target_path: PathBuf,
    working_dir: PathBuf,
    launch_url: String,
    icon_path: Option<PathBuf>,
    app_identifier: String,
) -> std::io::Result<()> {
    let output_path = windows_wide_path(&output_path);
    let target_path = windows_wide_path(&target_path);
    let working_dir = windows_wide_path(&working_dir);
    let launch_url = windows_wide_string(&launch_url);
    let icon_path = icon_path
        .as_deref()
        .map(windows_wide_path)
        .unwrap_or_else(|| target_path.clone());

    // SAFETY:
    // - COM is initialized for this blocking thread before any COM object is created.
    // - `_com` is declared before the COM interface values, so it is dropped
    //   after them and calls `CoUninitialize` only once they are released.
    // - Every PCWSTR points to a NUL-terminated UTF-16 buffer that lives until
    //   each call using it has returned.
    unsafe {
        let init_result = CoInitializeEx(
            None,
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        );
        windows_result(init_result.ok())?;
        let _com = WindowsComGuard;

        let shortcut: IShellLinkW = windows_result(CoCreateInstance(
            &ShellLink,
            None,
            CLSCTX_INPROC_SERVER,
        ))?;
        windows_result(shortcut.SetPath(windows_pcwstr(&target_path)))?;
        windows_result(shortcut.SetArguments(windows_pcwstr(&launch_url)))?;
        windows_result(
            shortcut.SetWorkingDirectory(windows_pcwstr(&working_dir)),
        )?;
        windows_result(
            shortcut.SetIconLocation(windows_pcwstr(&icon_path), 0),
        )?;

        let property_store: IPropertyStore = windows_result(shortcut.cast())?;
        let app_identifier = PROPVARIANT::from(app_identifier.as_str());
        windows_result(
            property_store.SetValue(&PKEY_AppUserModel_ID, &app_identifier),
        )?;
        windows_result(property_store.Commit())?;

        let persist_file: IPersistFile = windows_result(shortcut.cast())?;
        windows_result(persist_file.Save(windows_pcwstr(&output_path), true))?;
    }

    Ok(())
}

pub(super) fn refresh_shortcut_icon_cache(icon_path: &Path) {
    use windows::Win32::UI::Shell::{
        SHCNE_ASSOCCHANGED, SHCNE_UPDATEITEM, SHCNF_IDLIST, SHCNF_PATHW,
        SHChangeNotify,
    };

    let icon_path = windows_wide_path(icon_path);
    unsafe {
        SHChangeNotify(
            SHCNE_UPDATEITEM,
            SHCNF_PATHW,
            Some(icon_path.as_ptr().cast()),
            None,
        );
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}

fn windows_result<T>(result: windows::core::Result<T>) -> std::io::Result<T> {
    result.map_err(std::io::Error::other)
}

struct WindowsComGuard;

impl Drop for WindowsComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn windows_wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn windows_wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn windows_pcwstr(value: &[u16]) -> PCWSTR {
    PCWSTR::from_raw(value.as_ptr())
}
