use std::path::Path;

use serde_json::Value;
use tracing::debug;

pub struct InstanceInfo {
    pub vanilla_name: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
}

fn find_json(path: &Path) -> Option<(String, String)> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let primary = path.join(format!("{name}.json"));
    debug!(
        "instance_json: path={} looking for primary={}",
        path.display(),
        primary.display()
    );
    if primary.exists() {
        debug!(
            "instance_json: path={} json={} (by name match)",
            path.display(),
            primary.display()
        );
        let content = std::fs::read_to_string(&primary).ok()?;
        debug!(
            "instance_json: path={} primary content (len={}, first_200={:?})",
            path.display(),
            content.len(),
            &content[..content.len().min(200)]
        );
        return Some((name, content));
    }
    debug!(
        "instance_json: path={} primary={} NOT FOUND, enumerating directory",
        path.display(),
        primary.display()
    );
    let mut json_files = Vec::new();
    if let Ok(dir) = std::fs::read_dir(path) {
        for entry in dir.flatten() {
            let p = entry.path();
            debug!(
                "instance_json: path={} entry={}",
                path.display(),
                p.display()
            );
            if p.extension().map(|e| e == "json").unwrap_or(false) {
                json_files.push(p);
            }
        }
    }
    debug!(
        "instance_json: path={} found {} json files",
        path.display(),
        json_files.len()
    );
    if json_files.len() == 1 {
        debug!(
            "instance_json: path={} json={} (sole json fallback)",
            path.display(),
            json_files[0].display()
        );
        let content = std::fs::read_to_string(&json_files[0]).ok()?;
        debug!(
            "instance_json: path={} sole json content (len={}, first_200={:?})",
            path.display(),
            content.len(),
            &content[..content.len().min(200)]
        );
        let name = json_files[0]
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(name);
        return Some((name, content));
    }
    // Multiple JSONs: try each one, return the first with a valid version.
    // A single unreadable or malformed candidate must not abort the loop.
    for jf in &json_files {
        let Ok(content) = std::fs::read_to_string(jf) else {
            debug!(
                "instance_json: path={} json={} unreadable, trying next",
                path.display(),
                jf.display()
            );
            continue;
        };
        debug!(
            "instance_json: path={} trying json={} (len={}, first_200={:?})",
            path.display(),
            jf.display(),
            content.len(),
            &content[..content.len().min(200)]
        );
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
        else {
            debug!(
                "instance_json: path={} json={} invalid JSON, trying next",
                path.display(),
                jf.display()
            );
            continue;
        };
        let version = extract_version(&json, &content, None);
        if !version.is_empty() {
            let fname = jf
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| name.clone());
            debug!(
                "instance_json: path={} json={} (multiple-json pick, version={})",
                path.display(),
                jf.display(),
                version
            );
            return Some((fname, content));
        }
    }
    debug!(
        "instance_json: path={} multiple={} json files, none yielded a version",
        path.display(),
        json_files.len()
    );
    None
}

pub fn detect(path: &Path) -> Option<InstanceInfo> {
    let (name, content) = find_json(path)?;
    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            debug!("instance_json: path={} parse_err={}", path.display(), e);
            return None;
        }
    };
    let mut vanilla_name = extract_version(&json, &content, Some(&name));
    debug!(
        "instance_json: path={} extract_version returned {:?}",
        path.display(),
        vanilla_name
    );
    if vanilla_name.is_empty() {
        debug!(
            "instance_json: path={} version empty or Unknown",
            path.display()
        );
        return None;
    }
    vanilla_name = normalize_version(&vanilla_name);
    let loader = detect_loader(&content, &json).map(|(loader, version)| {
        let version = version.map(|version| {
            normalize_imported_loader_version(&loader, &vanilla_name, &version)
        });
        (loader, version)
    });
    debug!(
        "instance_json: path={} version={} loader={:?}",
        path.display(),
        vanilla_name,
        loader.as_ref().map(|(t, _)| t.as_str())
    );
    Some(InstanceInfo {
        vanilla_name,
        loader: loader.as_ref().map(|(t, _)| t.clone()),
        loader_version: loader.and_then(|(_, v)| v),
    })
}

fn normalize_version(raw: &str) -> String {
    let mut v = raw.to_string();
    if (v.starts_with("20.") || v.starts_with("21.")) && !v.starts_with("1.") {
        v = format!("1.{v}");
    }
    v = v.replace("_unobfuscated", "");
    v = v.replace(" Unobfuscated", "");
    v.trim().to_string()
}

pub(crate) fn normalize_imported_loader_version(
    loader: &str,
    game_version: &str,
    detected_version: &str,
) -> String {
    let detected_version = detected_version.trim();
    let without_family = match loader {
        "fabric" | "legacy_fabric" => detected_version
            .strip_prefix("fabric-loader-")
            .or_else(|| detected_version.strip_prefix("fabric-")),
        "quilt" => detected_version
            .strip_prefix("quilt-loader-")
            .or_else(|| detected_version.strip_prefix("quilt-")),
        "forge" => detected_version.strip_prefix("forge-"),
        "neoforge" => detected_version
            .strip_prefix("neoforge-")
            .or_else(|| detected_version.strip_prefix("neo-")),
        _ => None,
    }
    .unwrap_or(detected_version);

    match loader {
        "fabric" | "legacy_fabric" | "quilt" => without_family
            .strip_suffix(&format!("-{game_version}"))
            .unwrap_or(without_family)
            .to_string(),
        "forge" | "neoforge" => without_family
            .strip_prefix(&format!("{game_version}-"))
            .unwrap_or(without_family)
            .to_string(),
        _ => without_family.to_string(),
    }
}

fn extract_version(
    json: &Value,
    json_str: &str,
    folder_name: Option<&str>,
) -> String {
    // ① PCL download record clientVersion
    if let Some(v) = json.get("clientVersion").and_then(|v| v.as_str())
        && !v.is_empty()
    {
        debug!("extract_version: method=① clientVersion value={}", v);
        return v.to_string();
    }

    // ② HMCL patches[].version (id == "game")
    if let Some(patches) = json.get("patches").and_then(|v| v.as_array()) {
        for patch in patches {
            if patch.get("id").and_then(|v| v.as_str()) == Some("game")
                && let Some(ver) = patch.get("version").and_then(|v| v.as_str())
                && !ver.is_empty()
            {
                debug!(
                    "extract_version: method=② patches.game.version value={}",
                    ver
                );
                return ver.to_string();
            }
        }
    }

    // ③ arguments.game --fml.mcVersion (Forge/NeoForge)
    if let Some(args) = json
        .get("arguments")
        .and_then(|v| v.get("game"))
        .and_then(|v| v.as_array())
    {
        let mut mark = false;
        for arg in args {
            if mark && let Some(v) = arg.as_str() {
                debug!("extract_version: method=③ --fml.mcVersion value={}", v);
                return v.to_string();
            }
            if arg.as_str() == Some("--fml.mcVersion") {
                mark = true;
            }
        }
    }

    // ④ inheritsFrom (version inheritance) — must come before the `jar`
    // field, which is not always a version name.
    if let Some(v) = json.get("inheritsFrom").and_then(|v| v.as_str())
        && !v.is_empty()
    {
        debug!("extract_version: method=④ inheritsFrom value={}", v);
        return v.to_string();
    }

    // ⑤ libraries string regex fallback (Forge/OptiFine/FabricLike lib versions)
    // Use the original JSON string (from find_json) instead of re-serializing
    // the parsed Value, which would allocate a fresh string unnecessarily.
    if let Some(v) = extract_version_from_libraries(json_str) {
        debug!("extract_version: method=⑤ libraries value={}", v);
        return v;
    }

    // ⑥ JSON id field → extract leading version
    if let Some(id) = json.get("id").and_then(|v| v.as_str())
        && let Some(v) = extract_version_from_id(id)
    {
        debug!("extract_version: method=⑥ id id={} value={}", id, v);
        return v;
    }

    // ⑦ jar field (legacy versions store the base game in `jar`)
    if let Some(v) = json.get("jar").and_then(|v| v.as_str())
        && !v.is_empty()
    {
        debug!("extract_version: method=⑦ jar value={}", v);
        return v.to_string();
    }

    // ⑧ folder name fallback (renamed / non-standard instances)
    if let Some(name) = folder_name
        && let Some(v) = extract_version_from_id(name)
    {
        debug!("extract_version: method=⑧ folder_name value={}", v);
        return v;
    }

    debug!("extract_version: method=✗ all methods failed");
    String::new()
}

/// Extracts Minecraft version from library artifact coordinates in the JSON string.
/// Matches PCLCE's approach scanning for Forge/OptiFine/FabricLike lib entries.
/// Order: NeoForge before Forge (NeoForge JSON often also contains forge references).
fn extract_version_from_libraries(content: &str) -> Option<String> {
    // NeoForge: net.neoforged:neoforge:1.20.1-44.0.3 → "1.20.1"
    // Try known Maven coordinate formats (neoforge before forge).
    for needle in [
        "net.neoforged:neoforge:",
        "net.neoforged.neoforge:neoforge:",
        "net.neoforged.fml:modern:",
    ] {
        if let Some(pos) = content.find(needle) {
            let after = &content[pos + needle.len()..];
            if let Some(end) = after.find(&['"', ',', '\n', '}'] as &[char]) {
                let ver = &after[..end];
                if let Some(dash) = ver.find('-') {
                    return Some(ver[..dash].to_string());
                }
                return Some(ver.to_string());
            }
        }
    }
    // Forge: minecraftforge:forge:1.8.9-11.15.1.1722 → "1.8.9"
    //        net.minecraftforge:forge:1.21.1-52.0.0 (modern Forge, 1.13+)
    for needle in ["minecraftforge:forge:", "net.minecraftforge:forge:"] {
        if let Some(pos) = content.find(needle) {
            let after = &content[pos + needle.len()..];
            if let Some(end) = after.find(&['"', ',', '\n', '}'] as &[char]) {
                let ver = &after[..end];
                if let Some(dash) = ver.find('-') {
                    return Some(ver[..dash].to_string());
                }
                return Some(ver.to_string());
            }
        }
    }
    // OptiFine: optifine:OptiFine:1.8.9_HD_U_H5 → "1.8.9"
    if let Some(pos) = content.find("optifine:OptiFine:") {
        let after = &content[pos + "optifine:OptiFine:".len()..];
        if let Some(end) = after.find(&['"', ',', '\n', '}'] as &[char]) {
            let ver = &after[..end];
            if let Some(underscore) = ver.find('_') {
                return Some(ver[..underscore].to_string());
            }
            return Some(ver.to_string());
        }
    }
    // Fabric-like: net.fabricmc:fabric-loader:0.15.11-1.20.1 → "1.20.1"
    if let Some(pos) = content.find("net.fabricmc:fabric-loader:") {
        let after = &content[pos + "net.fabricmc:fabric-loader:".len()..];
        if let Some(end) = after.find(&['"', ',', '\n', '}'] as &[char]) {
            let ver = &after[..end];
            if let Some(dash) = ver.rfind('-') {
                return Some(ver[dash + 1..].to_string());
            }
        }
    }
    None
}

/// Extracts leading version number from the instance id.
/// e.g. "1.8.9-forge-11.15.1.1722" → "1.8.9"
/// Skips hash-like ids (≥32 chars, no separators).
fn extract_version_from_id(id: &str) -> Option<String> {
    let ver = id.trim();
    if ver.is_empty() {
        return None;
    }
    if ver.len() >= 32
        && !ver.contains('.')
        && !ver.contains('-')
        && !ver.contains('_')
    {
        return None;
    }
    if let Some(first_sep) = ver.find(['-', '_', ' ']) {
        let candidate = &ver[..first_sep];
        if candidate.starts_with("1.") || candidate.starts_with('2') {
            return Some(candidate.to_string());
        }
    }
    if ver.starts_with("1.") || ver.starts_with('2') {
        return Some(ver.to_string());
    }
    None
}

///参考自PCL启动器
fn detect_loader(
    content: &str,
    json: &Value,
) -> Option<(String, Option<String>)> {
    let lower = content.to_lowercase();

    // LabyMod
    if lower.contains("labymod_data") {
        let version = json
            .get("labymod_data")
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        return Some(("labymod".into(), version));
    }

    // Legacy Fabric
    if lower.contains("net.legacyfabric:intermediary") {
        let version = try_extract_version_from_needle(
            content,
            "net.fabricmc:fabric-loader:",
            None,
        );
        return Some(("legacy_fabric".into(), version));
    }

    // Fabric
    if lower.contains("net.fabricmc:fabric-loader") {
        let version = try_extract_version_from_needle(
            content,
            "net.fabricmc:fabric-loader:",
            None,
        );
        return Some(("fabric".into(), version));
    }

    // Quilt
    if lower.contains("org.quiltmc:quilt-loader") {
        let version = try_extract_version_from_needle(
            content,
            "org.quiltmc:quilt-loader:",
            None,
        );
        return Some(("quilt".into(), version));
    }

    // Cleanroom
    if lower.contains("com.cleanroommc:cleanroom:") {
        let version = try_extract_version_from_needle(
            content,
            "com.cleanroommc:cleanroom:",
            None,
        );
        return Some(("cleanroom".into(), version));
    }

    // Forge
    if lower.contains("minecraftforge") && !lower.contains("net.neoforge") {
        let version = try_extract_version_from_needle(
            content,
            "minecraftforge:forge:",
            None,
        )
        .or_else(|| {
            try_extract_version_from_needle(
                content,
                "net.minecraftforge:forge:",
                None,
            )
        })
        .or_else(|| {
            try_extract_version_from_needle(
                content,
                "net.minecraftforge:fmlloader:",
                None,
            )
        });
        return Some(("forge".into(), version));
    }

    // NeoForge
    if lower.contains("net.neoforge") {
        let version = try_extract_version_from_needle(
            content,
            "net.neoforged:neoforge:",
            None,
        )
        .or_else(|| {
            try_extract_version_from_needle(
                content,
                "net.neoforged.neoforge:neoforge:",
                None,
            )
        });
        return Some(("neoforge".into(), version));
    }

    // OptiFine
    if lower.contains("optifine") {
        let version = content
            .split("optifine:OptiFine:")
            .nth(1)
            .and_then(|s| {
                s.trim_matches(|c: char| c == '"' || c == ',' || c == ' ')
                    .split('_')
                    .next()
            })
            .map(|s| s.to_string());
        if version.is_some() {
            return Some(("optifine".into(), version));
        }
    }

    // LiteLoader
    if lower.contains("liteloader") {
        return Some(("lite_loader".into(), None));
    }

    debug!("detect_loader: no known loader library found in JSON content");
    None
}

/// Extracts the loader version string from JSON content by finding a needle
/// and reading until a terminator character.
fn try_extract_version_from_needle(
    content: &str,
    needle: &str,
    split_at: Option<char>,
) -> Option<String> {
    let pos = content.find(needle)?;
    let after = &content[pos + needle.len()..];
    let end = after.find(&['"', ',', '\n', '}'] as &[char])?;
    let ver = &after[..end];
    if let Some(ch) = split_at
        && let Some(pos) = ver.rfind(ch)
    {
        Some(ver[pos + 1..].to_string())
    } else {
        Some(ver.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect_from_json(content: &str) -> Option<(String, Option<String>)> {
        let json: Value = serde_json::from_str(content).expect("test JSON");
        detect_loader(content, &json)
    }

    fn assert_loader(
        content: &str,
        expected: &str,
        expected_version: Option<&str>,
    ) {
        let (loader, version) = detect_from_json(content)
            .unwrap_or_else(|| panic!("expected loader {expected}, got None"));
        assert_eq!(loader, expected);
        assert_eq!(version.as_deref(), expected_version);
    }

    #[test]
    fn test_forge() {
        assert_loader(
            r#"{
                "id": "1.21.1-forge-52.0.0",
                "libraries": [
                    {
                        "name": "net.minecraftforge:forge:1.21.1-52.0.0"
                    }
                ]
            }"#,
            "forge",
            Some("1.21.1-52.0.0"),
        );
    }

    #[test]
    fn normalizes_imported_loader_versions_for_central_resolution() {
        for (loader, game_version, detected, expected) in [
            ("fabric", "1.20.1", "0.15.11-1.20.1", "0.15.11"),
            ("quilt", "1.20.1", "0.26.4-1.20.1", "0.26.4"),
            (
                "forge",
                "1.7.10",
                "1.7.10-10.13.4.1614-1.7.10",
                "10.13.4.1614-1.7.10",
            ),
            ("forge", "1.20.1", "1.20.1-47.4.22", "47.4.22"),
            ("neoforge", "1.20.1", "1.20.1-44.0.3", "44.0.3"),
            ("neoforge", "1.21.4", "21.4.157", "21.4.157"),
        ] {
            assert_eq!(
                normalize_imported_loader_version(
                    loader,
                    game_version,
                    detected
                ),
                expected,
                "{loader} {game_version} {detected}"
            );
        }
    }

    #[test]
    fn test_neoforge() {
        assert_loader(
            r#"{
                "id": "1.20.1-neoforge-44.0.3",
                "libraries": [
                    {
                        "name": "net.neoforged:neoforge:1.20.1-44.0.3"
                    }
                ]
            }"#,
            "neoforge",
            Some("1.20.1-44.0.3"),
        );
    }

    #[test]
    fn test_fabric() {
        assert_loader(
            r#"{
                "id": "1.20.1-fabric-0.15.11",
                "libraries": [
                    {
                        "name": "net.fabricmc:fabric-loader:0.15.11-1.20.1"
                    }
                ]
            }"#,
            "fabric",
            Some("0.15.11-1.20.1"),
        );
    }

    #[test]
    fn test_quilt() {
        assert_loader(
            r#"{
                "id": "1.20.1-quilt-0.26.4",
                "libraries": [
                    {
                        "name": "org.quiltmc:quilt-loader:0.26.4-1.20.1"
                    }
                ]
            }"#,
            "quilt",
            Some("0.26.4-1.20.1"),
        );
    }

    #[test]
    fn test_optifine() {
        assert_loader(
            r#"{
                "id": "1.8.9-OptiFine",
                "libraries": [
                    {
                        "name": "optifine:OptiFine:1.8.9_HD_U_H5"
                    }
                ]
            }"#,
            "optifine",
            Some("1.8.9"),
        );
    }

    #[test]
    fn test_legacy_fabric() {
        assert_loader(
            r#"{
                "id": "1.8.9-legacy-fabric-0.13.1.4",
                "libraries": [
                    {
                        "name": "net.legacyfabric:intermediary:1.8.9"
                    },
                    {
                        "name": "net.fabricmc:fabric-loader:0.13.1.4-1.8.9"
                    }
                ]
            }"#,
            "legacy_fabric",
            Some("0.13.1.4-1.8.9"),
        );
    }

    #[test]
    fn test_cleanroom() {
        assert_loader(
            r#"{
                "id": "1.12.2-cleanroom-7.1.0",
                "libraries": [
                    {
                        "name": "com.cleanroommc:cleanroom:1.12.2-7.1.0"
                    }
                ]
            }"#,
            "cleanroom",
            Some("1.12.2-7.1.0"),
        );
    }

    #[test]
    fn test_labymod() {
        assert_loader(
            r#"{
                "id": "1.20.1-labymod",
                "labymod_data": {
                    "version": "4.4.20"
                }
            }"#,
            "labymod",
            Some("4.4.20"),
        );
    }

    #[test]
    fn test_lite_loader() {
        assert_loader(
            r#"{
                "id": "1.12.2-LiteLoader-1.12.2-SNAPSHOT",
                "libraries": [
                    {
                        "name": "com.mumfrey:liteloader:1.12.2"
                    }
                ]
            }"#,
            "lite_loader",
            None,
        );
    }

    #[test]
    fn test_no_loader() {
        assert!(detect_from_json(r#"{"id": "1.20.4"}"#).is_none());
    }

    #[test]
    fn test_detect_end_to_end() {
        let dir = tempfile::tempdir().expect("temp dir");
        let instance = dir.path().join(".minecraft");
        std::fs::create_dir(&instance).expect("create .minecraft dir");
        std::fs::write(
            instance.join(".minecraft.json"),
            r#"{
                "id": "1.20.1-fabric-0.15.11",
                "libraries": [
                    {
                        "name": "net.fabricmc:fabric-loader:0.15.11-1.20.1"
                    }
                ]
            }"#,
        )
        .expect("write instance json");

        let info = detect(&instance).expect("detect should succeed");
        assert_eq!(info.vanilla_name, "1.20.1");
        assert_eq!(info.loader.as_deref(), Some("fabric"));
        assert_eq!(info.loader_version.as_deref(), Some("0.15.11"));
    }
}
