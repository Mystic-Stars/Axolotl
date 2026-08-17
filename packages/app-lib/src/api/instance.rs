//! Theseus instance management interface

mod content;
mod export_mrpack;
mod get;
mod home;
mod install;
mod lifecycle;
mod paths;
mod projects;
mod run;

pub use self::content::{
    apply_content_update_plan, get_content_items, get_content_items_by_paths,
    get_content_snapshot, get_dependencies_as_content_items,
    get_install_candidates, get_installed_project_ids,
    get_linked_modpack_content, get_linked_modpack_info, get_projects,
    list_content_sets, plan_content_updates, refresh_content,
    sync_content_files,
};
pub use self::export_mrpack::{
    create_mrpack_json, export_mrpack, get_pack_export_candidates,
};
pub use self::get::{get, get_many, list};
pub use self::home::{
    get_daily_playtime, get_daily_playtime_details, set_pinned,
};
pub use self::install::get_optimal_jre_key;
pub(crate) use self::lifecycle::create;
pub use self::lifecycle::{cache_icon, edit, edit_icon, remove};
pub use self::paths::{get_full_path, get_mod_full_path};
pub(crate) use self::projects::emit_content_changed;
pub use self::projects::{
    ContentToggleResult, InstallProjectWithDependenciesRequest,
    add_project_from_path, add_project_from_version, import_world_save,
    install_datapack_bytes_to_world, install_datapack_to_world,
    install_project_with_dependencies, preview_project_with_dependencies,
    preview_project_with_dependencies_for_target, queue_curseforge_content,
    queue_curseforge_world, queue_project_with_dependencies,
    remove_content_entry, remove_project, repair_managed_modrinth,
    restore_pack_member_default, rollback_project,
    switch_content_entry_version, switch_project_version_with_dependencies,
    toggle_content_entries, toggle_content_entry, toggle_disable_project,
    update_all_projects, update_content_entry, update_managed_modrinth_version,
    update_project,
};
pub use self::run::{
    QuickPlayType, kill, run, run_with_extra_launch_args,
    try_update_playtime_by_instance_id,
};
pub use crate::state::{DailyPlaytime, DailyPlaytimeEntry};
