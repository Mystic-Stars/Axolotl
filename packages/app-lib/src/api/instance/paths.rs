use crate::state::State;
use crate::util::io;
use std::path::PathBuf;

#[tracing::instrument]
pub async fn get_full_path(instance_id: &str) -> crate::Result<PathBuf> {
    let state = State::get().await?;
    let (instance_path, game_dir_override) =
        crate::state::instances::adapters::sqlite::instance_rows::get_instance_path_and_game_dir_override_by_id(
            instance_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;

    Ok(io::canonicalize(state.directories.resolve_game_dir(
        &instance_path,
        game_dir_override.as_deref(),
    ))?)
}

#[tracing::instrument]
pub async fn get_mod_full_path(
    instance_id: &str,
    project_path: &str,
) -> crate::Result<PathBuf> {
    Ok(get_full_path(instance_id).await?.join(project_path))
}
