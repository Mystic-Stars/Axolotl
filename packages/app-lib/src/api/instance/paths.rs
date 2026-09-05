use crate::launcher::instance_runtime::InstanceRuntimeAdapter;
use crate::state::State;
use crate::util::io;
use std::path::{Path, PathBuf};

#[tracing::instrument]
pub async fn get_full_path(instance_id: &str) -> crate::Result<PathBuf> {
    let state = State::get().await?;
    let instance = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;

    // Directly associated instances never have a profile directory under
    // `profiles`; interactive APIs (open folder, worlds, servers, ...) must
    // operate on the linked `.minecraft` instead.
    let adapter = InstanceRuntimeAdapter::for_instance(
        &instance.instance,
        &state.directories,
    )?;
    Ok(io::canonicalize(adapter.game_dir())?)

    // `instance_game_dir` honours a per-instance `game_dir_override` before
    // falling back to the profile directory.
}

#[tracing::instrument]
pub async fn get_mod_full_path(
    instance_id: &str,
    project_path: &str,
) -> crate::Result<PathBuf> {
    Ok(get_full_path(instance_id).await?.join(project_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CreateDirectLinkInstance;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// The launcher state is a process-wide singleton; initialize it once and
    /// reuse it so `State::get()` resolves inside these APIs. The state root
    /// is intentionally leaked (`.keep()`) because the shared state outlives
    /// this function.
    async fn global_state() -> Arc<State> {
        if !State::initialized() {
            let root = TempDir::new().unwrap().keep();
            let _ =
                State::init_for_test(root.to_string_lossy().to_string()).await;
        }
        State::get().await.unwrap()
    }

    async fn create_direct_link_fixture(
        label: &str,
    ) -> (TempDir, crate::state::InstanceMetadata) {
        let state = global_state().await;
        let minecraft = TempDir::new().unwrap();
        let version_dir = minecraft.path().join("versions").join(label);
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(
            version_dir.join(format!("{label}.json")),
            serde_json::to_vec_pretty(&serde_json::json!({
                "id": label,
                "inheritsFrom": "1.20.1",
                "mainClass": "net.minecraft.client.main.Main"
            }))
            .unwrap(),
        )
        .unwrap();

        let instance = crate::state::create_direct_link_instance(
            CreateDirectLinkInstance {
                name: None,
                launcher_type:
                    crate::api::pack::import::ImportLauncherType::Generic,
                base_path: minecraft.path().to_path_buf(),
                instance_folder: format!("versions/{label}"),
                instance_path: None,
            },
            &state,
        )
        .await
        .unwrap();
        let metadata = crate::state::get_instance(&instance.id, &state.pool)
            .await
            .unwrap()
            .expect("metadata for created fixture");
        (minecraft, metadata)
    }

    #[tokio::test]
    async fn direct_link_instance_resolves_to_isolated_version_directory() {
        let state = global_state().await;
        let (_minecraft, metadata) =
            create_direct_link_fixture("paths-demo").await;

        let resolved = get_full_path(&metadata.instance.id).await.unwrap();

        assert!(resolved.ends_with(Path::new("versions").join("paths-demo")));
        assert!(
            !state
                .directories
                .instances_dir()
                .join(&metadata.instance.path)
                .exists(),
            "no profile directory may be created for a direct-link instance"
        );
    }

    #[tokio::test]
    async fn direct_link_file_browser_lists_linked_content() {
        let _state = global_state().await;
        let (minecraft, metadata) =
            create_direct_link_fixture("paths-browse").await;

        // The file browser lists directories relative to the resolved root.
        let version_dir =
            minecraft.path().join("versions").join("paths-browse");
        std::fs::create_dir_all(version_dir.join("mods")).unwrap();
        std::fs::write(
            version_dir.join("mods").join("browse-fixture.jar"),
            b"browse fixture",
        )
        .unwrap();

        let root = get_full_path(&metadata.instance.id).await.unwrap();
        let listed = root.join("mods").read_dir().unwrap();
        let names = listed
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec!["browse-fixture.jar".to_string()],
            "the browser root must expose the linked installation's content"
        );
    }

    #[tokio::test]
    async fn ordinary_instance_still_resolves_to_its_profile_directory() {
        let state = global_state().await;
        let metadata = crate::api::instance::create(
            format!("paths-normal {}", uuid::Uuid::new_v4()),
            "1.20.1".to_string(),
            crate::state::ModLoader::Vanilla,
            None,
            None,
            crate::state::InstanceLink::Unmanaged,
            None,
            None,
        )
        .await
        .unwrap();

        std::fs::create_dir_all(
            state
                .directories
                .instances_dir()
                .join(&metadata.instance.path),
        )
        .unwrap();

        let resolved = get_full_path(&metadata.instance.id).await.unwrap();
        assert_eq!(
            resolved,
            io::canonicalize(
                state
                    .directories
                    .instances_dir()
                    .join(&metadata.instance.path)
            )
            .unwrap()
        );
    }
}
