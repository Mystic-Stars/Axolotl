use theseus::content_search::ChineseSearchResolution;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("content-search")
        .invoke_handler(tauri::generate_handler![
            resolve_chinese_content_search,
        ])
        .build()
}

#[tauri::command]
pub fn resolve_chinese_content_search(
    query: String,
) -> ChineseSearchResolution {
    theseus::content_search::resolve_chinese_content_search(&query)
}
