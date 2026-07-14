use crate::api::Result;
use theseus::translation::{
    self, TranslationProvider, TranslationRequest, TranslationResponse,
    TranslationSettings,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("translation")
        .invoke_handler(tauri::generate_handler![
            translation_get_settings,
            translation_update_settings,
            translation_set_secret,
            translation_test_provider,
            translation_translate,
            translation_clear_cache,
        ])
        .build()
}

#[tauri::command]
pub async fn translation_get_settings() -> Result<TranslationSettings> {
    Ok(translation::get_settings().await?)
}

#[tauri::command]
pub async fn translation_update_settings(
    settings: TranslationSettings,
) -> Result<()> {
    Ok(translation::update_settings(settings).await?)
}

#[tauri::command]
pub async fn translation_set_secret(
    provider: TranslationProvider,
    secret: Option<String>,
) -> Result<()> {
    Ok(translation::set_secret(provider, secret).await?)
}

#[tauri::command]
pub async fn translation_test_provider(
    provider: TranslationProvider,
) -> Result<String> {
    Ok(translation::test_provider(provider).await?)
}

#[tauri::command]
pub async fn translation_translate(
    request: TranslationRequest,
) -> Result<TranslationResponse> {
    Ok(translation::translate(request).await?)
}

#[tauri::command]
pub async fn translation_clear_cache() -> Result<()> {
    Ok(translation::clear_cache().await?)
}
