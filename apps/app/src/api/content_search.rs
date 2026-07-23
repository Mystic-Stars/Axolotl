use theseus::content_search::{ChineseNameLookup, ChineseSearchResolution};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("content-search")
        .invoke_handler(tauri::generate_handler![
            resolve_chinese_content_search,
            lookup_chinese_content_names,
        ])
        .build()
}

#[tauri::command]
pub fn resolve_chinese_content_search(
    query: String,
) -> ChineseSearchResolution {
    theseus::content_search::resolve_chinese_content_search(&query)
}

#[tauri::command]
pub fn lookup_chinese_content_names(
    modrinth_slugs: Vec<String>,
    curseforge_slugs: Vec<String>,
) -> ChineseNameLookup {
    theseus::content_search::lookup_chinese_content_names(
        &modrinth_slugs,
        &curseforge_slugs,
    )
}
