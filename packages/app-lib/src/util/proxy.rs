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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProxyTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
}
