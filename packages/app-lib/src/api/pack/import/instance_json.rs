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
    if primary.exists() {
        debug!(
            "instance_json: path={} json={} (by name match)",
            path.display(),
            primary.display()
        );
        return std::fs::read_to_string(&primary).ok().map(|c| (name, c));
    }
    let mut json_files = Vec::new();
    if let Ok(dir) = std::fs::read_dir(path) {
        for entry in dir.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "json").unwrap_or(false) {
                json_files.push(p);
            }
        }
    }
    if json_files.len() == 1 {
        debug!(
            "instance_json: path={} json={} (sole json fallback)",
            path.display(),
            json_files[0].display()
        );
        let name = json_files[0]
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(name);
        return std::fs::read_to_string(&json_files[0])
            .ok()
            .map(|c| (name, c));
    }
    if json_files.len() > 1 {
        debug!(
            "instance_json: path={} multiple={} json files, can't pick one",
            path.display(),
            json_files.len()
        );
    }
    None
}

pub fn detect(path: &Path) -> Option<InstanceInfo> {
    let (_name, content) = find_json(path)?;
    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            debug!("instance_json: path={} parse_err={}", path.display(), e);
            return None;
        }
    };
    let mut vanilla_name = extract_version(&json, &content);
    if vanilla_name.is_empty() {
        debug!(
            "instance_json: path={} version empty or Unknown",
            path.display()
        );
        return None;
    }
    vanilla_name = normalize_version(&vanilla_name);
    let loader = detect_loader(&content, &json);
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

fn extract_version(json: &Value, json_str: &str) -> String {
    // ① PCL download record clientVersion
    if let Some(v) = json.get("clientVersion").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            return v.to_string();
        }
    }

    // ② HMCL patches[].version (id == "game")
    if let Some(patches) = json.get("patches").and_then(|v| v.as_array()) {
        for patch in patches {
            if patch.get("id").and_then(|v| v.as_str()) == Some("game") {
                if let Some(ver) = patch.get("version").and_then(|v| v.as_str())
                {
                    if !ver.is_empty() {
                        return ver.to_string();
                    }
                }
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
            if mark {
                if let Some(v) = arg.as_str() {
                    return v.to_string();
                }
            }
            if arg.as_str() == Some("--fml.mcVersion") {
                mark = true;
            }
        }
    }

    // ④ jar field (used with inheritsFrom in version inheritance chains)
    if let Some(v) = json.get("jar").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            return v.to_string();
        }
    }

    // ⑤ inheritsFrom (version inheritance)
    if let Some(v) = json.get("inheritsFrom").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            return v.to_string();
        }
    }

    // ⑥ libraries string regex fallback (Forge/OptiFine/FabricLike lib versions)
    // Use the original JSON string (from find_json) instead of re-serializing
    // the parsed Value, which would allocate a fresh string unnecessarily.
    if let Some(v) = extract_version_from_libraries(json_str) {
        return v;
    }

    // ⑦ JSON id field → extract leading version
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        if let Some(v) = extract_version_from_id(id) {
            return v;
        }
    }

    String::new()
}

/// Extracts Minecraft version from library artifact coordinates in the JSON string.
/// Matches PCLCE's approach scanning for Forge/OptiFine/FabricLike lib entries.
fn extract_version_from_libraries(content: &str) -> Option<String> {
    // Forge: minecraftforge:forge:1.8.9-11.15.1.1722 → "1.8.9"
    if let Some(pos) = content.find("minecraftforge:forge:") {
        let after = &content[pos + "minecraftforge:forge:".len()..];
        if let Some(end) = after.find(&['"', ',', '\n', '}'] as &[char]) {
            let ver = &after[..end];
            if let Some(dash) = ver.find('-') {
                return Some(ver[..dash].to_string());
            }
            return Some(ver.to_string());
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

/// Detects which mod loader is used and extracts its version.
/// Only loaders we can actually install (mapped to PackDependency) are detected.
fn detect_loader(
    content: &str,
    json: &Value,
) -> Option<(String, Option<String>)> {
    // Order per PCLCE: check Fabric/Quilt before Forge, neoforge before forge
    let loader_type = if content.contains("net.fabricmc:fabric-loader") {
        "fabric"
    } else if content.contains("org.quiltmc:quilt-loader") {
        "quilt"
    } else if content.contains("net.neoforge") {
        "neoforge"
    } else if content.contains("minecraftforge") {
        "forge"
    } else {
        return None;
    };

    let version = extract_loader_version(content, json, loader_type);
    Some((loader_type.to_string(), version))
}

fn extract_loader_version(
    content: &str,
    json: &Value,
    loader_type: &str,
) -> Option<String> {
    // First: parse from id field (e.g. "1.8.9-forge-11.15.1.1722")
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        if let Some(ver) = parse_loader_version_from_id(id, loader_type) {
            return Some(ver);
        }
    }

    // Second: extract from library entries
    let (needle, split_at) = match loader_type {
        "forge" => ("minecraftforge:forge:", Some('-')),
        "neoforge" => ("net.neoforged:neoforge:", None),
        "fabric" => ("net.fabricmc:fabric-loader:", None),
        "quilt" => ("org.quiltmc:quilt-loader:", None),
        _ => return None,
    };

    if let Some(pos) = content.find(needle) {
        let after = &content[pos + needle.len()..];
        if let Some(end) = after.find(&['"', ',', '\n', '}'] as &[char]) {
            let ver = &after[..end];
            if let Some(ch) = split_at {
                if let Some(pos) = ver.rfind(ch) {
                    return Some(ver[pos + 1..].to_string());
                }
            }
            return Some(ver.to_string());
        }
    }
    None
}

/// Parses loader version from the instance id.
/// e.g. "1.8.9-forge-11.15.1.1722" → "11.15.1.1722"
///      "1.20.1-fabric-0.15.11" → "0.15.11"
fn parse_loader_version_from_id(id: &str, loader_type: &str) -> Option<String> {
    let id_lower = id.to_lowercase();
    let keyword = match loader_type {
        "forge" => "forge",
        "neoforge" => "neoforge",
        "fabric" => "fabric",
        "quilt" => "quilt",
        _ => return None,
    };

    let pos = id_lower.find(keyword)?;
    let after = &id[pos + keyword.len()..];

    // Try "<keyword>-<version>" pattern (most common)
    let after_trimmed = after.trim_start_matches(['-', '_', ' ']);
    if !after_trimmed.is_empty()
        && after_trimmed.chars().next()?.is_ascii_digit()
    {
        // Take until next separator (or end of string)
        let end = after_trimmed
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(after_trimmed.len());
        if end > 0 {
            return Some(after_trimmed[..end].to_string());
        }
    }

    // Try "<keyword><digits.>" pattern (no separator, e.g. "Forge11.15.1")
    if let Some(first_digit) = after.find(|c: char| c.is_ascii_digit()) {
        let ver = &after[first_digit..];
        let end = ver
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(ver.len());
        if end > 0 {
            return Some(ver[..end].to_string());
        }
    }
    None
}
