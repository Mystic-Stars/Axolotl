use crate::api::Result;
use theseus::ai::{
    self, AiBulkModelUpdate, AiModelUpdate, AiProviderConfigUpdate,
    AiProviderDefinition, AiProviderModel, AiSettings, AiState,
    OAuthDeviceCode, OAuthPollStatus,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("ai")
        .invoke_handler(tauri::generate_handler![
            ai_get_catalog,
            ai_get_state,
            ai_update_settings,
            ai_update_provider,
            ai_set_api_key,
            ai_set_credential,
            ai_update_model,
            ai_remove_model,
            ai_update_models_bulk,
            ai_fetch_models,
            ai_test_provider,
            ai_begin_oauth,
            ai_poll_oauth,
            ai_disconnect_oauth,
        ])
        .build()
}

#[tauri::command]
pub fn ai_get_catalog() -> Vec<AiProviderDefinition> {
    ai::catalog()
}

#[tauri::command]
pub async fn ai_get_state() -> Result<AiState> {
    Ok(ai::get_state().await?)
}

#[tauri::command]
pub async fn ai_update_settings(settings: AiSettings) -> Result<()> {
    Ok(ai::update_settings(settings).await?)
}

#[tauri::command]
pub async fn ai_update_provider(update: AiProviderConfigUpdate) -> Result<()> {
    Ok(ai::update_provider(update).await?)
}

#[tauri::command]
pub fn ai_set_api_key(
    provider_id: String,
    secret: Option<String>,
) -> Result<()> {
    Ok(ai::set_api_key(provider_id, secret)?)
}

#[tauri::command]
pub fn ai_set_credential(
    provider_id: String,
    credential: String,
    secret: Option<String>,
) -> Result<()> {
    Ok(ai::set_credential(provider_id, credential, secret)?)
}

#[tauri::command]
pub async fn ai_update_model(update: AiModelUpdate) -> Result<()> {
    Ok(ai::update_model(update).await?)
}

#[tauri::command]
pub async fn ai_remove_model(
    provider_id: String,
    model_id: String,
) -> Result<()> {
    Ok(ai::remove_model(provider_id, model_id).await?)
}

#[tauri::command]
pub async fn ai_update_models_bulk(update: AiBulkModelUpdate) -> Result<()> {
    Ok(ai::update_models_bulk(update).await?)
}

#[tauri::command]
pub async fn ai_fetch_models(
    provider_id: String,
) -> Result<Vec<AiProviderModel>> {
    Ok(ai::fetch_models(provider_id).await?)
}

#[tauri::command]
pub async fn ai_test_provider(
    provider_id: String,
    model_id: String,
) -> Result<String> {
    Ok(ai::test_provider(provider_id, model_id).await?)
}

#[tauri::command]
pub async fn ai_begin_oauth(provider_id: String) -> Result<OAuthDeviceCode> {
    Ok(ai::begin_oauth(provider_id).await?)
}

#[tauri::command]
pub async fn ai_poll_oauth(flow_id: uuid::Uuid) -> Result<OAuthPollStatus> {
    Ok(ai::poll_oauth(flow_id).await?)
}

#[tauri::command]
pub fn ai_disconnect_oauth(provider_id: String) -> Result<()> {
    Ok(ai::disconnect_oauth(provider_id)?)
}
