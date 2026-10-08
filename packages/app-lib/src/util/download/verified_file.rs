//! Runtime proof that a verified digest belongs to an unchanged staged file.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(test)]
tokio::task_local! {
    static VERIFICATION_SCANS: std::cell::Cell<usize>;
}

#[cfg(test)]
pub(crate) fn record_scan() {
    let _ = VERIFICATION_SCANS.try_with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
pub(crate) async fn track_scans<F: std::future::Future>(
    future: F,
) -> (F::Output, usize) {
    VERIFICATION_SCANS
        .scope(std::cell::Cell::new(0), async {
            let output = future.await;
            (output, VERIFICATION_SCANS.with(std::cell::Cell::get))
        })
        .await
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedFile {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
    identity: (u64, u64),
    #[cfg(unix)]
    changed: (i64, i64),
}

impl VerifiedFile {
    pub(crate) async fn capture(path: &Path, size: u64) -> Option<Self> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path).ok()?;
            let metadata = file.metadata().ok()?;
            if !metadata.is_file() || metadata.len() != size {
                return None;
            }
            Some(Self {
                path,
                size,
                modified: metadata.modified().ok()?,
                created: metadata.created().ok(),
                identity: file_identity(&file, &metadata)?,
                #[cfg(unix)]
                changed: {
                    use std::os::unix::fs::MetadataExt;
                    (metadata.ctime(), metadata.ctime_nsec())
                },
            })
        })
        .await
        .ok()
        .flatten()
    }

    pub(crate) async fn matches(&self, path: &Path, size: u64) -> bool {
        cfg!(unix)
            && self.path == path
            && self.size == size
            && Self::capture(path, size).await.as_ref() == Some(self)
    }
}

#[cfg(windows)]
fn file_identity(
    file: &std::fs::File,
    _metadata: &std::fs::Metadata,
) -> Option<(u64, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe {
        GetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            &raw mut information,
        )
        .ok()?;
    }
    Some((
        information.dwVolumeSerialNumber as u64,
        (information.nFileIndexHigh as u64) << 32
            | information.nFileIndexLow as u64,
    ))
}

#[cfg(unix)]
fn file_identity(
    _file: &std::fs::File,
    metadata: &std::fs::Metadata,
) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((metadata.dev(), metadata.ino()))
}

#[cfg(not(any(windows, unix)))]
fn file_identity(
    _file: &std::fs::File,
    _metadata: &std::fs::Metadata,
) -> Option<(u64, u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn proof_rejects_different_path_size_and_modified_file() {
        let directory = tempfile::tempdir().unwrap();
        let staged = directory.path().join("staged");
        let other = directory.path().join("other");
        tokio::fs::write(&staged, b"original").await.unwrap();
        tokio::fs::write(&other, b"original").await.unwrap();
        let proof = VerifiedFile::capture(&staged, 8).await.unwrap();
        assert_eq!(proof.matches(&staged, 8).await, cfg!(unix));
        assert!(!proof.matches(&other, 8).await);
        assert!(!proof.matches(&staged, 7).await);
        let file = std::fs::File::options().write(true).open(&staged).unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(123456),
        ))
        .unwrap();
        assert!(!proof.matches(&staged, 8).await);
    }

    #[tokio::test]
    async fn proof_rejects_same_length_edits_with_restored_mtime() {
        use std::io::{Seek, Write};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("staged");
        std::fs::write(&path, b"original").unwrap();
        let proof = VerifiedFile::capture(&path, 8).await.unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut file =
            std::fs::File::options().write(true).open(&path).unwrap();
        file.rewind().unwrap();
        file.write_all(b"modified").unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        assert!(!proof.matches(&path, 8).await);
    }
}
