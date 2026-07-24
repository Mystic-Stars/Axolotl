use std::path::{Path, PathBuf};

const PORTABLE_DIR_NAME: &str = ".Axolotl";

/// 在启动时初始化便携模式
/// 检查 `.Axolotl` 文件夹是否存在且可写
/// 如果存在且可写，将 `THESEUS_CONFIG_DIR` 环境变量设置为该路径
pub fn init_portable_mode() {
    if let Some(portable_dir) = get_portable_dir() {
        if is_dir_writable(&portable_dir) {
            // SAFETY: 在 main() 函数开头调用，此时还没有其他线程访问环境变量
            unsafe {
                std::env::set_var("THESEUS_CONFIG_DIR", &portable_dir);
            }
            tracing::info!(
                "Portable mode enabled: THESEUS_CONFIG_DIR={}",
                portable_dir.display()
            );
        } else {
            tracing::warn!(
                "Portable directory {} exists but is not writable",
                portable_dir.display()
            );
        }
    }
}

/// 获取便携模式目录路径（如果存在）
/// 返回 `Some(PathBuf)` 如果 `.Axolotl` 文件夹存在于可执行文件的父目录中，否则返回 `None`
fn get_portable_dir() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;
    let app_dir = exe_path.parent()?;
    let portable_dir = app_dir.join(PORTABLE_DIR_NAME);
    portable_dir.is_dir().then_some(portable_dir)
}

/// 检查目录是否可写
fn is_dir_writable(path: &Path) -> bool {
    let temp_path = path.join(".tmp_write_test");
    let result = std::fs::write(&temp_path, "test").is_ok();
    if result {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

/// Tauri 命令：检查应用程序是否运行在便携模式下
#[tauri::command]
pub fn is_portable_mode() -> bool {
    std::env::var_os("THESEUS_CONFIG_DIR").is_some()
}
