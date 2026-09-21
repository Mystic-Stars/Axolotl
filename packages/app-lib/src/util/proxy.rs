use reqwest::{ClientBuilder, Proxy};
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyMode {
    /// Bypass all proxies and connect directly.
    None,
    #[default]
    System,
    Custom,
}

impl ProxyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::System => "system",
            Self::Custom => "custom",
        }
    }

    pub fn from_string(value: &str) -> Self {
        match value {
            "none" => Self::None,
            "custom" => Self::Custom,
            _ => Self::System,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub mode: ProxyMode,
    pub url: String,
    pub username: String,
    pub password: String,
}

impl ProxyConfig {
    pub fn storage_key() -> &'static str {
        "proxy_config_v1"
    }

    pub fn custom_url_trimmed(&self) -> Option<&str> {
        let trimmed = self.url.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    pub fn has_credentials(&self) -> bool {
        !self.username.trim().is_empty()
    }

    /// Return the standard JVM properties needed by child Java processes to
    /// use the configured custom proxy. System proxies are intentionally not
    /// converted here because Java cannot reliably discover the platform
    /// proxy configuration from portable JVM arguments.
    pub fn java_args(&self) -> Vec<String> {
        if self.mode != ProxyMode::Custom {
            return Vec::new();
        }
        let Some(url) = self.custom_url_trimmed() else {
            return Vec::new();
        };
        let Ok(parsed) = reqwest::Url::parse(url) else {
            return Vec::new();
        };
        let Some(host) = parsed.host_str() else {
            return Vec::new();
        };
        let scheme = parsed.scheme();
        let default_port = match scheme {
            "http" => 80,
            "https" => 443,
            "socks4" | "socks5" | "socks5h" => 1080,
            _ => return Vec::new(),
        };
        let port = parsed.port().unwrap_or(default_port);
        let mut args = Vec::new();
        if scheme.starts_with("socks") {
            args.push(format!("-DsocksProxyHost={host}"));
            args.push(format!("-DsocksProxyPort={port}"));
        } else {
            args.push(format!("-Dhttp.proxyHost={host}"));
            args.push(format!("-Dhttp.proxyPort={port}"));
            args.push(format!("-Dhttps.proxyHost={host}"));
            args.push(format!("-Dhttps.proxyPort={port}"));
        }
        args
    }

    pub fn validate(&self) -> Result<()> {
        if self.mode != ProxyMode::Custom {
            return Ok(());
        }
        let Some(url) = self.custom_url_trimmed() else {
            return Err(crate::ErrorKind::InputError(
                "Custom proxy URL is required when mode is Custom".to_string(),
            )
            .into());
        };
        let parsed_url = reqwest::Url::parse(url).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Proxy URL is invalid: {error}"
            ))
        })?;
        let scheme = parsed_url.scheme().to_string();
        if !matches!(
            scheme.as_str(),
            "http" | "https" | "socks4" | "socks5" | "socks5h"
        ) {
            return Err(crate::ErrorKind::InputError(format!(
                "Unsupported proxy scheme '{scheme}'. Use http, https, socks5, or socks5h."
            ))
            .into());
        }
        reqwest::Proxy::all(url).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Proxy URL is invalid: {error}"
            ))
        })?;
        Ok(())
    }

    pub fn apply(&self, builder: ClientBuilder) -> Result<ClientBuilder> {
        match self.mode {
            ProxyMode::None => Ok(builder.no_proxy()),
            ProxyMode::System => Ok(builder),
            ProxyMode::Custom => {
                let url = self.custom_url_trimmed().ok_or_else(|| {
                    crate::ErrorKind::InputError(
                        "Custom proxy URL is required when mode is Custom"
                            .to_string(),
                    )
                })?;
                let parsed_url = reqwest::Url::parse(url).map_err(|error| {
                    crate::ErrorKind::InputError(format!(
                        "Proxy URL is invalid: {error}"
                    ))
                })?;
                let proxy = if self.has_credentials() {
                    if parsed_url.scheme().starts_with("socks") {
                        let mut authed_url = parsed_url;
                        let _ = authed_url.set_username(&self.username);
                        let _ = authed_url.set_password(Some(&self.password));
                        Proxy::all(authed_url.as_str()).map_err(|error| {
                            crate::ErrorKind::InputError(format!(
                                "Proxy URL is invalid: {error}"
                            ))
                        })?
                    } else {
                        Proxy::all(url)
                            .map_err(|error| {
                                crate::ErrorKind::InputError(format!(
                                    "Proxy URL is invalid: {error}"
                                ))
                            })?
                            .basic_auth(&self.username, &self.password)
                    }
                } else {
                    Proxy::all(url).map_err(|error| {
                        crate::ErrorKind::InputError(format!(
                            "Proxy URL is invalid: {error}"
                        ))
                    })?
                };
                Ok(builder.proxy(proxy))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(url: &str) -> ProxyConfig {
        ProxyConfig {
            mode: ProxyMode::Custom,
            url: url.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn java_args_map_http_proxy_to_http_and_https() {
        assert_eq!(
            custom("http://127.0.0.1:7890").java_args(),
            vec![
                "-Dhttp.proxyHost=127.0.0.1",
                "-Dhttp.proxyPort=7890",
                "-Dhttps.proxyHost=127.0.0.1",
                "-Dhttps.proxyPort=7890",
            ]
        );
    }

    #[test]
    fn java_args_use_default_port_for_socks_proxy() {
        assert_eq!(
            custom("socks5://127.0.0.1").java_args(),
            vec!["-DsocksProxyHost=127.0.0.1", "-DsocksProxyPort=1080"]
        );
    }

    #[test]
    fn java_args_are_empty_without_custom_proxy() {
        assert!(ProxyConfig::default().java_args().is_empty());
        assert!(custom("").java_args().is_empty());
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProxyTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
}
