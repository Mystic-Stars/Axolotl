//! Platform-related code
use daedalus::minecraft::{Os, OsRule};

/// (e.g. "Run as administrator" on Windows).
#[cfg(target_os = "windows")]
pub fn is_process_elevated() -> bool {
    use std::ffi::c_void;
    use std::ptr::null_mut;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn OpenProcessToken(
            process_handle: *mut c_void,
            desired_access: u32,
            token_handle: *mut *mut c_void,
        ) -> i32;
        fn GetTokenInformation(
            token_handle: *mut c_void,
            token_information_class: u32,
            token_information: *mut c_void,
            token_information_length: u32,
            return_length: *mut u32,
        ) -> i32;
    }

    const TOKEN_QUERY: u32 = 0x8;
    const TOKEN_ELEVATION_TYPE: u32 = 18;
    const TOKEN_ELEVATION_TYPE_FULL: u32 = 2;

    // SAFETY: GetCurrentProcess returns a pseudo-handle that must not be
    // closed. The token handle is queried and closed on every path.
    unsafe {
        let process = GetCurrentProcess();
        let mut token = null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &raw mut token) == 0
            || token.is_null()
        {
            return false;
        }

        let mut elevation_type: u32 = 0;
        let mut size = 0u32;
        let success = GetTokenInformation(
            token,
            TOKEN_ELEVATION_TYPE,
            &raw mut elevation_type as *mut c_void,
            std::mem::size_of::<u32>() as u32,
            &raw mut size,
        ) != 0;
        let _ = CloseHandle(token);
        success && elevation_type == TOKEN_ELEVATION_TYPE_FULL
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_process_elevated() -> bool {
    false
}

// Bit width
#[cfg(target_pointer_width = "64")]
pub const ARCH_WIDTH: &str = "64";

#[cfg(target_pointer_width = "32")]
pub const ARCH_WIDTH: &str = "32";

// Platform rule handling
pub fn os_rule(
    rule: &OsRule,
    java_arch: &str,
    // Minecraft updated over 1.18.2 (supports MacOS Natively)
    minecraft_updated: bool,
) -> bool {
    let mut rule_match = true;

    if let Some(ref arch) = rule.arch {
        rule_match &= !matches!(arch.as_str(), "x86" | "arm");
    }

    if let Some(name) = &rule.name {
        if minecraft_updated
            && !matches!(name, &Os::LinuxArm64 | &Os::LinuxArm32)
        {
            rule_match &= Os::native() == name.get_os()
                || &Os::native_arch(java_arch) == name;
        } else {
            rule_match &= &Os::native_arch(java_arch) == name;
        }
    }

    // `rule.version` is ignored because it's not usually seen on real recent
    // Minecraft version manifests, its alleged regex syntax is undefined and is
    // likely to not match `Regex`'s, and the way to get the value to match it
    // against is allegedly calling `System.getProperty("os.version")`, which
    // on Windows the OpenJDK implements by fetching the kernel32.dll version,
    // an approach that no public Rust library implements. Moreover, launchers
    // such as PrismLauncher also ignore this field. Code references:
    // - https://github.com/openjdk/jdk/blob/948ade8e7003a41683600428c8e3155c7ed798db/src/java.base/windows/native/libjava/java_props_md.c#L556
    // - https://github.com/PrismLauncher/PrismLauncher/blob/1c20faccf88999474af70db098a4c10e7a03af33/launcher/minecraft/Rule.h#L77
    // - https://github.com/FillZpp/sys-info-rs/blob/60ecf1470a5b7c90242f429934a3bacb6023ec4d/c/windows.c#L23-L38

    rule_match
}

pub fn classpath_separator(java_arch: &str) -> &'static str {
    match Os::native_arch(java_arch) {
        Os::Osx
        | Os::OsxArm64
        | Os::Linux
        | Os::LinuxArm32
        | Os::LinuxArm64
        | Os::Unknown => ":",
        Os::Windows | Os::WindowsArm64 => ";",
    }
}
