use crate::api::Result;
use theseus::content_favorites::{
    self, ContentFavorite, ContentFavoriteInput, ContentFavoriteProvider,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("content-favorites")
        .invoke_handler(tauri::generate_handler![
            content_favorites_list,
            content_favorites_add,
            content_favorites_remove,
        ])
        .build()
}

#[tauri::command]
pub async fn content_favorites_list() -> Result<Vec<ContentFavorite>> {
    Ok(content_favorites::list().await?)
}

#[tauri::command]
pub async fn content_favorites_add(
    favorite: ContentFavoriteInput,
) -> Result<ContentFavorite> {
    Ok(content_favorites::add(favorite).await?)
}

#[tauri::command]
pub async fn content_favorites_remove(
    provider: ContentFavoriteProvider,
    project_id: String,
) -> Result<()> {
    content_favorites::remove(provider, &project_id).await?;
    Ok(())
}
