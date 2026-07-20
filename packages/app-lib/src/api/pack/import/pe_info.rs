#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    pub fn get_product_name(path: &Path) -> Option<String> {
        use windows::Win32::Storage::FileSystem::{
            GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
        };
        use windows::core::PCWSTR;

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut unused = 0u32;
        let size = unsafe {
            GetFileVersionInfoSizeW(
                PCWSTR::from_raw(wide.as_ptr()),
                Some(&mut unused),
            )
        };
        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let ok = unsafe {
            GetFileVersionInfoW(
                PCWSTR::from_raw(wide.as_ptr()),
                Some(0),
                size,
                buffer.as_mut_ptr() as *mut _,
            )
        };
        if ok.is_err() {
            return None;
        }

        let sub = OsStr::new("\\VarFileInfo\\Translation")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>();

        let mut lang_ptr = std::ptr::null_mut::<std::ffi::c_void>();
        let mut lang_len = 0u32;

        let ok = unsafe {
            VerQueryValueW(
                buffer.as_ptr() as *const _,
                PCWSTR::from_raw(sub.as_ptr()),
                &mut lang_ptr,
                &mut lang_len,
            )
        };
        if ok.0 == 0 || lang_len == 0 || lang_ptr.is_null() {
            return None;
        }

        // lang_len from VerQueryValueW(Translation) is in bytes, not characters
        let lang_len_u16 = (lang_len / 2) as usize;
        if lang_len_u16 < 2 {
            return None;
        }
        let lang = unsafe {
            std::slice::from_raw_parts(lang_ptr as *const u16, lang_len_u16)
        };
        let block = format!(
            "\\StringFileInfo\\{:04x}{:04x}\\ProductName",
            lang[0], lang[1]
        );
        let block_wide: Vec<u16> = OsStr::new(&block)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut val_ptr = std::ptr::null_mut::<std::ffi::c_void>();
        let mut val_len = 0u32;

        let ok = unsafe {
            VerQueryValueW(
                buffer.as_ptr() as *const _,
                PCWSTR::from_raw(block_wide.as_ptr()),
                &mut val_ptr,
                &mut val_len,
            )
        };
        if ok.0 == 0 || val_len == 0 || val_ptr.is_null() {
            return None;
        }

        let slice = unsafe {
            std::slice::from_raw_parts(
                val_ptr as *const u16,
                val_len as usize - 1,
            )
        };
        String::from_utf16(slice).ok()
    }

    pub fn folder_has_product(base_path: &Path, product_name: &str) -> bool {
        if !base_path.is_dir() {
            return false;
        }
        let read_dir = match std::fs::read_dir(base_path) {
            Ok(d) => d,
            Err(_) => return false,
        };
        for entry in read_dir.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "exe").unwrap_or(false) {
                if let Some(product) = get_product_name(&p) {
                    if product.contains(product_name) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use std::path::Path;
    pub fn folder_has_product(_base_path: &Path, _product_name: &str) -> bool {
        false
    }
}

pub use imp::folder_has_product;
