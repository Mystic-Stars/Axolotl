//! Modrinth CDN redirect normalization for native download requests.

use url::Url;

use super::modrinth_cdn;

const TIANPAO_HOST: &str = "mod.tianpao.top";

/// Identifies a Tianpao cache miss, including retired CDN URLs that require
/// normalization before a subsequent request.
pub(crate) fn is_tianpao_official_redirect(
    current: &Url,
    location: Option<&str>,
) -> bool {
    current
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case(TIANPAO_HOST))
        && location.is_some_and(|location| {
            let Ok(redirect) = current.join(location) else {
                return false;
            };
            modrinth_cdn::is_current_url(&modrinth_cdn::normalize_parsed(
                redirect,
            ))
        })
}

pub(crate) fn repair_official_redirect(
    original: &Url,
    redirect: &Url,
    location: &str,
) -> Option<Url> {
    let mut repaired = modrinth_cdn::normalize_parsed(redirect.clone());
    if !location.is_ascii()
        && modrinth_cdn::is_current_url(&repaired)
        && !original.path().is_empty()
    {
        repaired.set_path(original.path());
        repaired.set_query(original.query());
        repaired.set_fragment(original.fragment());
        return Some(repaired);
    }
    (repaired != *redirect).then_some(repaired)
}

pub(crate) fn is_official_redirect(location: Option<&str>) -> bool {
    let Some(location) = location.filter(|location| {
        location.len() <= 8 * 1024
            && location.is_ascii()
            && location
                .get(..8)
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
    }) else {
        return false;
    };
    Url::parse(location).is_ok_and(|url| {
        modrinth_cdn::is_current_url(&modrinth_cdn::normalize_parsed(url))
    })
}
