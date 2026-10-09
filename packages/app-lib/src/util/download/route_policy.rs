//! Pure route classification helpers used by the download coordinator.

use crate::util::fetch::ResourceClass;
use url::Url;

pub(crate) fn is_official_modrinth_download_url(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        super::modrinth_cdn::is_current_url(&url)
            || (url.scheme() == "https"
                && url.host_str() == Some("api.modrinth.com"))
    })
}

pub(crate) fn is_official_version_manifest_url(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        matches!(
            url.host_str(),
            Some("piston-meta.mojang.com" | "launchermeta.mojang.com")
        ) && url.path().contains("version_manifest")
    })
}

/// Loader Maven repositories served mirror-first while retaining an official
/// fallback for content the mirrors have not synced yet.
pub(crate) fn uses_mirror_first_loader_routes(
    url: &str,
    resource: ResourceClass,
) -> bool {
    if !matches!(
        resource,
        ResourceClass::MinecraftLibrary | ResourceClass::Loader
    ) {
        return false;
    }
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if matches!(
        url.host_str(),
        Some(
            "maven.minecraftforge.net"
                | "maven.fabricmc.net"
                | "maven.neoforged.net"
        )
    ) {
        return true;
    }
    let path = url.path().to_ascii_lowercase();
    ["minecraftforge", "fabricmc", "neoforged"]
        .iter()
        .any(|loader| path.contains(loader))
}
