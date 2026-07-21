use std::{collections::HashMap, path::PathBuf};

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

fn find_config(base_path: &PathBuf) -> Option<PathBuf> {
    let path = base_path.join(".hmcl").join("hmcl.json");
    if path.exists() {
        return Some(path);
    }
    None
}

pub fn config_exists(base_path: &PathBuf) -> bool {
    find_config(base_path).is_some()
}

pub fn get_instances(base_path: &PathBuf) -> Vec<(String, String)> {
    let config_path = match find_config(base_path) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let config: HmclConfig = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
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
    base_path: &PathBuf,
    instance_key: &str,
) -> Option<String> {
    let config_path = find_config(base_path)?;
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: HmclConfig = serde_json::from_str(&content).ok()?;

    for (key, entry) in &config.configurations {
        if key == instance_key {
            let game_dir = PathBuf::from(&entry.game_dir);
            let resolved = if game_dir.is_absolute() {
                game_dir
            } else {
                base_path.join(&game_dir)
            };
            if resolved.is_dir() {
                return Some(entry.game_dir.clone());
            }
        }
    }
    None
}
