use crate::api::Result;
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LinuxUpdateInfo {
	pub is_linux: bool,
	pub distribution: Option<String>,
	pub package_manager: Option<String>,
	pub package_manager_label: Option<String>,
	pub update_command: Option<String>,
	pub alternate_command: Option<String>,
	pub notes: Vec<String>,
	pub needs_terminal: bool,
}

#[tauri::command]
pub fn get_linux_update_info() -> LinuxUpdateInfo {
	#[cfg(not(target_os = "linux"))]
	{
		return LinuxUpdateInfo {
			is_linux: false,
			distribution: None,
			package_manager: None,
			package_manager_label: None,
			update_command: None,
			alternate_command: None,
			notes: vec![],
			needs_terminal: false,
		};
	}

	#[cfg(target_os = "linux")]
	{
		let distribution = detect_distribution();
		let package_manager = detect_package_manager();
		let (label, update_command, alternate_command, notes, needs_terminal) =
			get_commands_and_notes(&distribution, &package_manager);

		LinuxUpdateInfo {
			is_linux: true,
			distribution,
			package_manager,
			package_manager_label: label,
			update_command,
			alternate_command,
			notes,
			needs_terminal,
		}
	}
}

#[tauri::command]
pub fn execute_package_manager_update(command: String) -> Result<()> {
	open_in_terminal(&command)
}

#[cfg(not(target_os = "linux"))]
fn open_in_terminal(_command: &str) -> Result<()> {
	Err(theseus::Error::from(theseus::ErrorKind::OtherError(
		"Not supported on this platform".to_string(),
	))
	.into())
}

#[cfg(target_os = "linux")]
fn open_in_terminal(command: &str) -> Result<()> {
	let terminals: &[(&str, &[&str])] = &[
		("x-terminal-emulator", &["-e", "sh", "-c"]),
		("gnome-terminal", &["--", "bash", "-c"]),
		("konsole", &["-e", "bash", "-c"]),
		("xfce4-terminal", &["-e", "bash", "-c"]),
		("alacritty", &["-e", "bash", "-c"]),
		("kitty", &["bash", "-c"]),
		("xterm", &["-e", "bash", "-c"]),
	];

	for (term, prefix_args) in terminals {
		if command_exists(term) {
			let full_command = format!(
				"{}; echo; echo 'Press Enter to close...'; read",
				command
			);
			let mut cmd = std::process::Command::new(term);
			for arg in *prefix_args {
				cmd.arg(arg);
			}
			cmd.arg(&full_command);
			cmd.spawn().map_err(|e| {
				theseus::Error::from(theseus::ErrorKind::OtherError(format!(
					"Failed to launch terminal: {e}"
				)))
			})?;
			return Ok(());
		}
	}

	Err(theseus::Error::from(theseus::ErrorKind::OtherError(
		"No terminal emulator found. Please run the command manually.".to_string(),
	))
	.into())
}

#[cfg(target_os = "linux")]
fn detect_distribution() -> Option<String> {
	if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
		for line in content.lines() {
			if let Some(id) = line.strip_prefix("ID=") {
				let id = id.trim_matches('"');
				if !id.is_empty() {
					return Some(id.to_string());
				}
			}
		}
	}

	if let Ok(output) = std::process::Command::new("lsb_release")
		.arg("-si")
		.output()
	{
		if output.status.success() {
			let id = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
			if !id.is_empty() {
				return Some(id);
			}
		}
	}

	None
}

#[cfg(target_os = "linux")]
fn command_exists(cmd: &str) -> bool {
	std::process::Command::new("which")
		.arg(cmd)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.map(|s| s.success())
		.unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn detect_package_manager() -> Option<String> {
	if command_exists("apt") {
		return Some("apt".to_string());
	}
	if command_exists("yay") {
		return Some("yay".to_string());
	}
	if command_exists("paru") {
		return Some("paru".to_string());
	}
	if command_exists("pacman") {
		return Some("pacman".to_string());
	}

	None
}

#[cfg(target_os = "linux")]
fn get_commands_and_notes(
	distribution: &Option<String>,
	pm: &Option<String>,
) -> (
	Option<String>,
	Option<String>,
	Option<String>,
	Vec<String>,
	bool,
) {
	let is_arch = pm.as_deref() == Some("pacman")
		|| pm.as_deref() == Some("yay")
		|| pm.as_deref() == Some("paru")
		|| distribution
			.as_deref()
			.map(|d| d.contains("arch") || d.contains("manjaro"))
			.unwrap_or(false);

	match pm.as_deref() {
		Some("apt") => (
			Some("APT".to_string()),
			Some("pkexec sh -c \"curl -fsSL https://ppa.axlmc.org/setup.sh | bash && apt update && apt install -y axolotl-launcher\" && { notify-send 'Axolotl Launcher' '更新完成，请重启应用' || notify-send 'Axolotl Launcher' 'Update complete. Please restart the app.' || echo 'Update complete. Please restart.'; }".to_string()),
			None,
			vec![],
			true,
		),
		Some("yay") | Some("paru") | Some("pacman") => {
			let helper = pm.as_deref().unwrap_or("yay");
			(
				Some("AUR".to_string()),
				Some(format!("{} -S axolotl-launcher-bin && {{ notify-send 'Axolotl Launcher' '更新完成，请重启应用' || notify-send 'Axolotl Launcher' 'Update complete. Please restart the app.' || echo 'Update complete. Please restart.'; }}", helper)),
				Some(format!("{} -S axolotl-launcher && {{ notify-send 'Axolotl Launcher' '更新完成，请重启应用' || notify-send 'Axolotl Launcher' 'Update complete. Please restart the app.' || echo 'Update complete. Please restart.'; }}", helper)),
				vec![
					"预编译二进制包，安装更快（推荐）".to_string(),
					"从源码构建，速度较慢".to_string(),
				],
				true,
			)
		}
		_ => {
			if is_arch {
				(
					Some("AUR".to_string()),
					Some("yay -S axolotl-launcher-bin && { notify-send 'Axolotl Launcher' '更新完成，请重启应用' || notify-send 'Axolotl Launcher' 'Update complete. Please restart the app.' || echo 'Update complete. Please restart.'; }".to_string()),
					Some("yay -S axolotl-launcher && { notify-send 'Axolotl Launcher' '更新完成，请重启应用' || notify-send 'Axolotl Launcher' 'Update complete. Please restart the app.' || echo 'Update complete. Please restart.'; }".to_string()),
					vec![
						"预编译二进制包，安装更快（推荐）".to_string(),
						"从源码构建，速度较慢".to_string(),
					],
					true,
				)
			} else {
				(None, None, None, vec![], false)
			}
		}
	}
}
