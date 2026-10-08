//! Modrinth CDN redirect normalization for native download requests.

use url::Url;

use crate::util::fetch::{
    MODRINTH_CDN_LEGACY_HOST, MODRINTH_CDN_OFFICIAL_HOST,
};

const TIANPAO_HOST: &str = "mod.tianpao.top";

/// Identifies a Tianpao response that points at one of Modrinth's real CDN
/// hosts. The caller uses this only to abandon the mirror request; it must
/// follow the server-provided host without rewriting it.
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
            redirect.host_str().is_some_and(|host| {
                host.eq_ignore_ascii_case(MODRINTH_CDN_LEGACY_HOST)
                    || host.eq_ignore_ascii_case(MODRINTH_CDN_OFFICIAL_HOST)
            })
        })
}

pub(crate) fn repair_official_redirect(
    original: &Url,
    redirect: &Url,
    location: &str,
) -> Option<Url> {
    if location.is_ascii()
        || !redirect.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case(MODRINTH_CDN_LEGACY_HOST)
                || host.eq_ignore_ascii_case(MODRINTH_CDN_OFFICIAL_HOST)
        })
        || original.path().is_empty()
    {
        return None;
    }
    let mut repaired = redirect.clone();
    repaired.set_path(original.path());
    repaired.set_query(original.query());
    repaired.set_fragment(original.fragment());
    Some(repaired)
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
    let authority = location[8..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    authority.eq_ignore_ascii_case(MODRINTH_CDN_OFFICIAL_HOST)
        || authority
            .eq_ignore_ascii_case(&format!("{MODRINTH_CDN_OFFICIAL_HOST}:443"))
        || authority.eq_ignore_ascii_case(MODRINTH_CDN_LEGACY_HOST)
        || authority
            .eq_ignore_ascii_case(&format!("{MODRINTH_CDN_LEGACY_HOST}:443"))
}
