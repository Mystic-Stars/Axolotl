//! Download attempt diagnostics and bounded retry history.

use crate::ErrorKind;
use crate::util::fetch::{
    DownloadRoute, DownloadRouteSource, MAX_DOWNLOAD_ATTEMPT_HISTORY,
    MAX_DOWNLOAD_DIAGNOSTIC_BYTES, ProxyPolicy, route_host,
    sanitize_url_for_log,
};
use reqwest::StatusCode;
use std::collections::VecDeque;

#[derive(Debug)]
pub(crate) struct DownloadAttemptDiagnostic {
    pub(crate) attempt: usize,
    pub(crate) source: DownloadRouteSource,
    pub(crate) url: String,
    pub(crate) proxy: ProxyPolicy,
    pub(crate) dns_candidates: Vec<std::net::IpAddr>,
    pub(crate) remote_addr: Option<std::net::SocketAddr>,
    pub(crate) http_version: Option<reqwest::Version>,
    pub(crate) status: Option<u16>,
    pub(crate) category: &'static str,
    pub(crate) decision: &'static str,
    pub(crate) detail: String,
}

pub(crate) fn bounded_diagnostic_text(
    value: impl AsRef<str>,
    max_chars: usize,
) -> String {
    value.as_ref().chars().take(max_chars).collect()
}

pub(crate) fn download_error_category(error: &crate::Error) -> &'static str {
    match error.raw.as_ref() {
        ErrorKind::FetchError(source) => {
            let detail = format!("{source:?}").to_ascii_lowercase();
            if source.status().is_some() {
                "http"
            } else if source.is_timeout() && source.is_body() {
                "stall"
            } else if source.is_timeout() {
                "timeout"
            } else if source.is_connect()
                && ["certificate", "tls", "ssl"]
                    .iter()
                    .any(|needle| detail.contains(needle))
            {
                "tls"
            } else if source.is_connect()
                && ["dns", "lookup", "resolve"]
                    .iter()
                    .any(|needle| detail.contains(needle))
            {
                "dns"
            } else if source.is_connect() {
                "connect"
            } else {
                "network"
            }
        }
        ErrorKind::NetworkError(message) => {
            if message.contains("no response received") {
                "timeout"
            } else {
                "network"
            }
        }
        ErrorKind::LabrinthError(_) | ErrorKind::HttpError { .. } => "http",
        ErrorKind::HashError(_, _) => "integrity",
        ErrorKind::JSONError(_) => "integrity",
        ErrorKind::IOError(_) | ErrorKind::StdIOError(_) => "io",
        ErrorKind::OtherError(message) => {
            let message = message.to_ascii_lowercase();
            if message.contains("content-range") || message.contains("range") {
                "range"
            } else if message.contains("integrity")
                || message.contains("checksum")
                || message.contains("validation")
            {
                "integrity"
            } else if message.contains("truncated") {
                "stall"
            } else {
                "other"
            }
        }
        _ => "other",
    }
}

pub(crate) fn download_error_detail(error: &crate::Error) -> String {
    match error.raw.as_ref() {
        ErrorKind::FetchError(source) => source.status().map_or_else(
            || format!("{} failure", download_error_category(error)),
            |status| format!("HTTP {}", status.as_u16()),
        ),
        ErrorKind::LabrinthError(error) => error.status.map_or_else(
            || "API response failure".to_string(),
            |status| format!("HTTP {status}"),
        ),
        ErrorKind::HttpError { status, .. } => format!("HTTP {status}"),
        ErrorKind::HashError(_, _) => "hash mismatch".to_string(),
        ErrorKind::JSONError(_) => "JSON validation failed".to_string(),
        ErrorKind::IOError(_) | ErrorKind::StdIOError(_) => {
            "I/O failure".to_string()
        }
        ErrorKind::OtherError(_) | ErrorKind::NetworkError(_) => {
            format!("{} failure", download_error_category(error))
        }
        _ => bounded_diagnostic_text(error.to_string(), 256),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_download_attempt_diagnostic(
    history: &mut VecDeque<DownloadAttemptDiagnostic>,
    route: &DownloadRoute,
    attempt: usize,
    category: &'static str,
    decision: &'static str,
    detail: impl AsRef<str>,
    status: Option<StatusCode>,
    remote_addr: Option<std::net::SocketAddr>,
    http_version: Option<reqwest::Version>,
) {
    if history.len() == MAX_DOWNLOAD_ATTEMPT_HISTORY {
        history.pop_front();
    }
    let dns_candidates = route_host(route)
        .map(|host| {
            super::proxy_context::resolver(route.proxy)
                .resolved_addresses(&host)
        })
        .unwrap_or_default()
        .into_iter()
        .take(8)
        .collect();
    history.push_back(DownloadAttemptDiagnostic {
        attempt,
        source: route.source,
        url: bounded_diagnostic_text(sanitize_url_for_log(&route.url), 512),
        proxy: route.proxy,
        dns_candidates,
        remote_addr,
        http_version,
        status: status.map(|status| status.as_u16()),
        category,
        decision,
        detail: bounded_diagnostic_text(detail, 256),
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_download_attempt_failure(
    history: &mut VecDeque<DownloadAttemptDiagnostic>,
    route: &DownloadRoute,
    attempt: usize,
    error: &crate::Error,
    decision: &'static str,
    status: Option<StatusCode>,
    remote_addr: Option<std::net::SocketAddr>,
    http_version: Option<reqwest::Version>,
) {
    push_download_attempt_diagnostic(
        history,
        route,
        attempt,
        download_error_category(error),
        decision,
        download_error_detail(error),
        status,
        remote_addr,
        http_version,
    );
}

pub(crate) fn attach_download_attempt_history(
    error: crate::Error,
    history: &VecDeque<DownloadAttemptDiagnostic>,
    attempts: usize,
    attempt_budget: usize,
) -> crate::Error {
    let mut context = format!(
        "Download failed after {attempts}/{attempt_budget} attempts. Recent attempt history:"
    );
    for item in history {
        let line = format!(
            "\n- attempt={}; source={}; url={}; proxy={:?}; dns={:?}; remote={:?}; http={:?}; status={:?}; category={}; decision={}; detail={}",
            item.attempt,
            item.source.as_str(),
            item.url,
            item.proxy,
            item.dns_candidates,
            item.remote_addr,
            item.http_version,
            item.status,
            item.category,
            item.decision,
            item.detail,
        );
        if context.len() + line.len() > MAX_DOWNLOAD_DIAGNOSTIC_BYTES {
            context.push_str("\n- older diagnostic details omitted");
            break;
        }
        context.push_str(&line);
    }
    error.with_context(context)
}
