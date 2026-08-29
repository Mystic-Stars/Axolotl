use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HmclConfig {
    configurations: HashMap<String, HmclConfiguration>,
}

#[derive(Debug, Deserialize)]
struct HmclConfiguration {
    #[serde(rename = "gameDir")]
    game_dir: String,
}

fn find_config(base_path: &Path) -> Option<std::path::PathBuf> {
    let path = base_path.join(".hmcl").join("hmcl.json");
    if path.exists() {
        return Some(path);
    }
    None
}

pub fn config_exists(base_path: &Path) -> bool {
    find_config(base_path).is_some()
}

pub fn get_instances(base_path: &Path) -> Vec<(String, String)> {
    let Some(config_path) = find_config(base_path) else {
        return Vec::new();
    };

    let Ok(content) = std::fs::read_to_string(&config_path) else {
        tracing::warn!(
            "hmcl: failed to read config at {}",
            config_path.display()
        );
        return Vec::new();
    };

    let config: HmclConfig = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "hmcl: failed to parse config at {}: {e}",
                config_path.display()
            );
            return Vec::new();
        }
    };

    let mut instances = Vec::new();
    for (key, entry) in &config.configurations {
        let game_dir = PathBuf::from(&entry.game_dir);
        let resolved = if game_dir.is_absolute() {
            game_dir
        } else {
            base_path.join(&game_dir)
        };
        if resolved.is_dir() {
            instances.push((key.clone(), entry.game_dir.clone()));
        }
    }
    instances.sort_by(|a, b| a.0.cmp(&b.0));
    instances
}

pub fn get_instance_path(
    base_path: &Path,
    instance_key: &str,
) -> Option<String> {
    // Reuse get_instances() to avoid parsing the config file twice.
    get_instances(base_path)
        .into_iter()
        .find(|(key, _)| key == instance_key)
        .map(|(_, path)| path)
}
