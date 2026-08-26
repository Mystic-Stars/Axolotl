//! Reading, writing, and downloading files inside a server's directory.

use std::path::{Path, PathBuf};

use futures::StreamExt;
use sha1_smol::Sha1;

use crate::event::ServerPayloadType;
use crate::event::emit::emit_server;
use crate::util::io::{self, IOError};
use crate::{ErrorKind, Result};

use super::manifest::server_path;

const DOWNLOAD_PROGRESS_STEP: u64 = 512 * 1024;

pub async fn read_file(server_id: &str, file: &str) -> Result<String> {
    let path = resolve_server_file(server_id, file).await?;
    let bytes = io::read(&path).await?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(text)
}

pub async fn write_file(
    server_id: &str,
    file: &str,
    contents: &str,
) -> Result<()> {
    let path = resolve_server_file(server_id, file).await?;
    io::write(&path, contents).await?;
    Ok(())
}

pub async fn download_file(
    server_id: &str,
    url: &str,
    filename: &str,
    expected_sha1: Option<String>,
) -> Result<()> {
    let dir = server_path(server_id).await?;
    let destination = safe_join(&dir, filename)?;
    let partial = destination.with_extension("part");

    let client = reqwest::Client::builder()
        .user_agent(crate::launcher_user_agent())
        .build()
        .map_err(|e| ErrorKind::NetworkError(e.to_string()))?;
    let response = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())?;
    let total = response.content_length();
    let mut stream = response.bytes_stream();

    let mut file = tokio::fs::File::create(&partial)
        .await
        .map_err(|e| IOError::with_path(e, &partial))?;
    let mut hasher = Sha1::new();
    let mut downloaded: u64 = 0;
    let mut last_reported: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| IOError::with_path(e, &partial))?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if downloaded - last_reported >= DOWNLOAD_PROGRESS_STEP {
            last_reported = downloaded;
            emit_server(
                server_id,
                ServerPayloadType::DownloadProgress { downloaded, total },
            )
            .await
            .ok();
        }
    }
    drop(file);

    if let Some(expected) = expected_sha1.as_deref() {
        let actual = hasher.digest().to_string();
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&partial).await;
            return Err(ErrorKind::NetworkError(format!(
                "Download checksum mismatch for {filename}: expected {expected}, got {actual}"
            ))
            .as_error());
        }
    }

    tokio::fs::rename(&partial, &destination)
        .await
        .map_err(|e| IOError::with_path(e, &destination))?;
    emit_server(
        server_id,
        ServerPayloadType::DownloadProgress {
            downloaded,
            total: Some(downloaded.max(total.unwrap_or(0))),
        },
    )
    .await
    .ok();
    Ok(())
}

async fn resolve_server_file(server_id: &str, file: &str) -> Result<PathBuf> {
    let dir = server_path(server_id).await?;
    safe_join(&dir, file)
}

fn safe_join(dir: &Path, file: &str) -> Result<PathBuf> {
    if file.is_empty()
        || file.contains('\\')
        || file.starts_with('/')
        || file
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
    {
        return Err(ErrorKind::InputError(format!(
            "Invalid file name: {file}"
        ))
        .as_error());
    }
    Ok(dir.join(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_rejects_traversal() {
        let dir = Path::new("/tmp/servers/a");
        assert!(safe_join(dir, "server.properties").is_ok());
        assert!(safe_join(dir, "../secret").is_err());
        assert!(safe_join(dir, "/etc/passwd").is_err());
        assert!(safe_join(dir, "a//b").is_err());
        assert!(safe_join(dir, "").is_err());
    }
}
