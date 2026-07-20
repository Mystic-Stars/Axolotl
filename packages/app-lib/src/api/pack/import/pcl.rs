use std::path::PathBuf;

use serde::Deserialize;

#[cfg(target_os = "windows")]
pub fn read_pcl_registry() -> Option<String> {
	use winreg::enums::HKEY_CURRENT_USER;
	let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
	let key = hkcu.open_subkey("SOFTWARE\\PCL").ok()?;
	key.get_value("LaunchFolders").ok()
}

#[cfg(not(target_os = "windows"))]
pub fn read_pcl_registry() -> Option<String> {
	None
}

#[derive(Debug, Deserialize)]
struct PclCeConfig {
	#[serde(rename = "LaunchFolders")]
	launch_folders: Option<String>,
}

fn read_pclce_config() -> Option<String> {
	let path = dirs::data_dir()?.join("PCLCE").join("config.v1.json");
	let content = std::fs::read_to_string(&path).ok()?;
	let config: PclCeConfig = serde_json::from_str(&content).ok()?;
	config.launch_folders
}

fn parse_pcl_folders(raw: &str) -> Vec<(String, String)> {
	let mut result = Vec::new();
	for entry in raw.split('|') {
		let entry = entry.trim();
		if entry.is_empty() {
			continue;
		}
		if let Some((name, path)) = entry.split_once('>') {
			let path = PathBuf::from(path.trim());
			if path.is_dir() {
				result.push((name.trim().to_string(), path.to_string_lossy().to_string()));
			}
		}
	}
	result.sort_by(|a, b| a.0.cmp(&b.0));
	result
}

pub fn config_exists() -> bool {
	read_pclce_config().is_some()
}

pub fn get_pcl_instances() -> Vec<(String, String)> {
	let raw = read_pcl_registry().unwrap_or_default();
	parse_pcl_folders(&raw)
}

pub fn get_pclce_instances() -> Vec<(String, String)> {
	let raw = read_pclce_config().unwrap_or_default();
	parse_pcl_folders(&raw)
}

pub fn get_pcl_instance_path(instance_name: &str) -> Option<String> {
	for (name, path) in get_pcl_instances() {
		if name == instance_name {
			return Some(path);
		}
	}
	None
}

pub fn get_pclce_instance_path(instance_name: &str) -> Option<String> {
	for (name, path) in get_pclce_instances() {
		if name == instance_name {
			return Some(path);
		}
	}
	None
}
