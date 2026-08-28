//! Platform utilities for inspecting and terminating processes that hold a
//! TCP port, used to resolve "port already in use" conflicts.

use serde::Serialize;
use tokio::process::Command;

use crate::{ErrorKind, Result};

pub async fn kill_port_process(port: u16) -> Result<()> {
    let pids = port_listener_pids(port).await?;
    if pids.is_empty() {
        return Err(ErrorKind::InputError(format!(
            "No process found listening on port {port}"
        ))
        .as_error());
    }
    for pid in pids {
        force_terminate_pid(pid).await?;
    }
    Ok(())
}

#[derive(Serialize, Debug, Clone)]
pub struct PortProcessInfo {
    pub pid: u32,
    pub name: Option<String>,
}

/// Returns the first process listening on the given TCP port, if any.
pub async fn port_process(port: u16) -> Result<Option<PortProcessInfo>> {
    let pids = port_listener_pids(port).await?;
    let Some(&pid) = pids.first() else {
        return Ok(None);
    };
    Ok(Some(PortProcessInfo {
        pid,
        name: process_name(pid).await,
    }))
}

#[cfg(not(target_os = "windows"))]
async fn port_listener_pids(port: u16) -> Result<Vec<u32>> {
    let output = Command::new("lsof")
        .args(["-t", "-i", &format!("tcp:{port}"), "-s", "tcp:listen"])
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to look up processes listening on port {port}: {e}"
            ))
            .as_error()
        })?;
    let mut pids: Vec<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect();
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

#[cfg(not(target_os = "windows"))]
async fn force_terminate_pid(pid: u32) -> Result<()> {
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to terminate process {pid}: {e}"
            ))
            .as_error()
        })?;
    if !output.status.success() {
        return Err(ErrorKind::LauncherError(format!(
            "Failed to terminate process {pid}"
        ))
        .as_error());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn process_name(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(target_os = "windows")]
async fn port_listener_pids(port: u16) -> Result<Vec<u32>> {
    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to look up processes listening on port {port}: {e}"
            ))
            .as_error()
        })?;
    let mut pids: Vec<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 5
                || !columns[3].eq_ignore_ascii_case("LISTENING")
            {
                return None;
            }
            let local_address = columns[1];
            let local_port = local_address.rsplit(':').next()?;
            (local_port == port.to_string())
                .then(|| columns[4].parse::<u32>().ok())?
        })
        .collect();
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

#[cfg(target_os = "windows")]
async fn force_terminate_pid(pid: u32) -> Result<()> {
    let output = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to terminate process {pid}: {e}"
            ))
            .as_error()
        })?;
    if !output.status.success() {
        return Err(ErrorKind::LauncherError(format!(
            "Failed to terminate process {pid}"
        ))
        .as_error());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
async fn process_name(pid: u32) -> Option<String> {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?;
    if line.starts_with("INFO") {
        return None;
    }
    let name = line.split(',').next()?.trim_matches('"').to_string();
    (!name.is_empty()).then_some(name)
}
