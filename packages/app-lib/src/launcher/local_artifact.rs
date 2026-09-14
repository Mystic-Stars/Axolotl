//! Single-pass verification and copy of artifacts from an external runtime.

use crate::util::fetch::{self, IoSemaphore};
use crate::util::io;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) async fn candidate_is_usable(
    local: Option<&super::download::LocalRuntimeSource>,
    relative_path: &Path,
    expected_size: Option<u64>,
) -> crate::Result<bool> {
    let Some(local) = local else {
        return Ok(false);
    };
    let candidate = local.root.join(relative_path);
    let metadata = match tokio::fs::metadata(candidate).await {
        Ok(metadata) => metadata,
        Err(_) => return Ok(false),
    };
    Ok(metadata.is_file()
        && expected_size.is_none_or(|size| metadata.len() == size))
}

/// Copy a local artifact through a temporary sibling while calculating its
/// SHA-1 from the exact bytes written. Returns `false` for a missing, wrong-
/// sized, or hash-mismatched source so the caller can use the network path.
pub(crate) async fn copy_verified(
    source: &Path,
    destination: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
    semaphore: &IoSemaphore,
) -> crate::Result<bool> {
    let destination_lock = fetch::destination_download_lock(destination);
    let _destination_guard = destination_lock.lock().await;
    let _permit = semaphore.0.acquire().await?;
    let metadata = match tokio::fs::metadata(source).await {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) | Err(_) => return Ok(false),
    };
    if expected_size.is_some_and(|size| metadata.len() != size) {
        return Ok(false);
    }
    let Some(expected_sha1) = expected_sha1 else {
        return Ok(false);
    };

    if let Some(parent) = destination.parent() {
        io::create_dir_all(parent).await?;
    }
    let part_path = fetch::suffixed_path(destination, ".part");
    let mut input = match File::open(source).await {
        Ok(input) => input,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(false);
        }
        Err(error) => return Err(error.into()),
    };
    let mut output = File::create(&part_path).await?;
    let mut hasher = sha1_smol::Sha1::new();
    let mut copied = 0_u64;
    let mut buffer = vec![0_u8; 256 * 1024];
    let copy_result = async {
        loop {
            let read = input.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            output.write_all(&buffer[..read]).await?;
            copied += read as u64;
        }
        output.flush().await?;
        Ok::<_, std::io::Error>(())
    }
    .await;
    if let Err(error) = copy_result {
        let _ = tokio::fs::remove_file(&part_path).await;
        return Err(error.into());
    }

    let actual_sha1 = hasher.digest().to_string();
    if expected_size.is_some_and(|size| copied != size)
        || !actual_sha1.eq_ignore_ascii_case(expected_sha1)
    {
        let _ = tokio::fs::remove_file(&part_path).await;
        return Ok(false);
    }
    if let Err(error) = fetch::finalize_download(&part_path, destination).await
    {
        let _ = tokio::fs::remove_file(&part_path).await;
        return Err(error);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fetch::IoSemaphore;
    use tokio::sync::Semaphore;

    #[tokio::test]
    async fn verified_copy_writes_and_hashes_in_one_pass() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("nested/destination");
        tokio::fs::write(&source, b"asset").await.unwrap();
        let semaphore = IoSemaphore(Semaphore::new(1));

        assert!(
            copy_verified(
                &source,
                &destination,
                Some("05fac94380a70241f23780e7aef62b190894238f"),
                Some(5),
                &semaphore,
            )
            .await
            .unwrap()
        );
        assert_eq!(tokio::fs::read(&destination).await.unwrap(), b"asset");
        assert!(!fetch::suffixed_path(&destination, ".part").exists());
    }

    #[tokio::test]
    async fn failed_hash_removes_partial_and_destination() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");
        tokio::fs::write(&source, b"asset").await.unwrap();
        let semaphore = IoSemaphore(Semaphore::new(1));

        assert!(
            !copy_verified(
                &source,
                &destination,
                Some("00"),
                Some(5),
                &semaphore
            )
            .await
            .unwrap()
        );
        assert!(!destination.exists());
        assert!(!fetch::suffixed_path(&destination, ".part").exists());
    }

    #[tokio::test]
    async fn concurrent_verified_copies_share_one_destination_safely() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");
        tokio::fs::write(&source, b"asset").await.unwrap();
        let semaphore = IoSemaphore(Semaphore::new(2));

        let first = copy_verified(
            &source,
            &destination,
            Some("05fac94380a70241f23780e7aef62b190894238f"),
            Some(5),
            &semaphore,
        );
        let second = copy_verified(
            &source,
            &destination,
            Some("05fac94380a70241f23780e7aef62b190894238f"),
            Some(5),
            &semaphore,
        );

        let (first, second) = tokio::join!(first, second);
        assert!(first.unwrap());
        assert!(second.unwrap());
        assert_eq!(tokio::fs::read(&destination).await.unwrap(), b"asset");
        assert!(!fetch::suffixed_path(&destination, ".part").exists());
    }
}
