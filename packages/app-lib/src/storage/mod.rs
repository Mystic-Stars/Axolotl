//! Launcher storage scanning.
//!
//! Walks the launcher data directories and produces a summary tree of disk
//! usage. Symbolic links / junctions are never followed for the "actual"
//! counter; instead their referenced size is tracked separately and exposed
//! as the `symlink` portion of every node.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::state::{DirectoryInfo, InstanceMetadata};

pub const STORAGE_CACHE_VERSION: u32 = 1;
const STORAGE_CACHE_FILE: &str = "launcher-storage.cbor";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageNodeType {
    Instances,
    Cache,
    Meta,
    Database,
    Other,
    Instance,
    Mods,
    Replay,
    Resourcepacks,
    Saves,
    World,
    Schematics,
    Screenshots,
    Shaderpacks,
    Minimap,
    #[serde(rename = "distant-horizons")]
    DistantHorizons,
    #[serde(rename = "db-file")]
    DbFile,
    #[serde(rename = "db-backup")]
    DbBackup,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoragePathKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoragePath {
    pub path: String,
    pub kind: StoragePathKind,
}

#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq,
)]
pub struct StorageSize {
    pub actual: u64,
    pub symlink: u64,
}

impl StorageSize {
    pub fn total(self) -> u64 {
        self.actual.saturating_add(self.symlink)
    }
}

impl std::ops::Add for StorageSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            actual: self.actual.saturating_add(rhs.actual),
            symlink: self.symlink.saturating_add(rhs.symlink),
        }
    }
}

impl std::ops::AddAssign for StorageSize {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for StorageSize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            actual: self.actual.saturating_sub(rhs.actual),
            symlink: self.symlink.saturating_sub(rhs.symlink),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: StorageNodeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "instance_id", skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    pub size: StorageSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    pub paths: Vec<StoragePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<StorageNode>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageTree {
    pub version: u32,
    #[serde(rename = "scannedAt")]
    pub scanned_at: DateTime<Utc>,
    pub total: StorageSize,
    pub categories: Vec<StorageNode>,
    #[serde(rename = "rootOther")]
    pub root_other: Option<StorageNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StorageCacheFile {
    version: u32,
    roots: String,
    scanned_at: DateTime<Utc>,
    tree: StorageTree,
}

#[derive(Clone, Copy, Default)]
struct Stats {
    actual: u64,
    symlink: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScanMode {
    Host,
    Reference,
}

impl From<Stats> for StorageSize {
    fn from(value: Stats) -> Self {
        Self {
            actual: value.actual,
            symlink: value.symlink,
        }
    }
}

fn to_size(stats: Stats) -> StorageSize {
    StorageSize {
        actual: stats.actual,
        symlink: stats.symlink,
    }
}

fn node_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn path_id(path: &Path, suffix: &str) -> String {
    format!("{}-{suffix}", node_path(path))
}

fn dirs() -> crate::Result<DirectoryInfo> {
    DirectoryInfo::global_handle_if_ready()
        .map(|directory| DirectoryInfo {
            settings_dir: directory.settings_dir.clone(),
            config_dir: directory.config_dir.clone(),
            app_identifier: directory.app_identifier.clone(),
        })
        .ok_or_else(|| {
            crate::ErrorKind::FSError(
                "Launcher state is not ready for storage scanning".to_string(),
            )
            .into()
        })
}

#[async_recursion]
async fn scan_path(
    path: &Path,
    mode: ScanMode,
    visited: &mut HashSet<PathBuf>,
) -> Stats {
    let mut stats = Stats::default();
    let mut entries = match fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(error) => {
            tracing::debug!(
                path = %path.display(),
                error = %error,
                "Cannot read directory while scanning storage; skipping"
            );
            return stats;
        }
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let child = entry.path();
        let meta = match fs::symlink_metadata(&child).await {
            Ok(meta) => meta,
            Err(error) => {
                tracing::debug!(
                    path = %child.display(),
                    error = %error,
                    "Cannot stat path while scanning storage; skipping"
                );
                continue;
            }
        };

        if crate::util::io::is_symlink_or_reparse(&meta) {
            stats.symlink = stats
                .symlink
                .saturating_add(referenced_size(&child, visited).await);
        } else if meta.is_dir() {
            let child_stats = scan_path(&child, mode, visited).await;
            stats += child_stats;
        } else if meta.is_file() {
            match mode {
                ScanMode::Host => {
                    stats.actual = stats.actual.saturating_add(meta.len());
                }
                ScanMode::Reference => {
                    stats.symlink = stats.symlink.saturating_add(meta.len());
                }
            }
        }
    }

    stats
}

impl std::ops::AddAssign for Stats {
    fn add_assign(&mut self, rhs: Self) {
        self.actual = self.actual.saturating_add(rhs.actual);
        self.symlink = self.symlink.saturating_add(rhs.symlink);
    }
}

#[async_recursion]
async fn referenced_stats(
    link: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Stats {
    let target = match fs::canonicalize(link).await {
        Ok(target) => target,
        Err(error) => {
            tracing::debug!(
                path = %link.display(),
                error = %error,
                "Cannot resolve symlink target while scanning storage"
            );
            return Stats::default();
        }
    };

    let meta = match fs::symlink_metadata(&target).await {
        Ok(meta) => meta,
        Err(_) => return Stats::default(),
    };

    if meta.is_file() {
        return Stats {
            actual: 0,
            symlink: meta.len(),
        };
    }

    if !visited.insert(target.clone()) {
        return Stats::default();
    }

    let stats = scan_path(&target, ScanMode::Reference, visited).await;
    visited.remove(&target);
    stats
}

async fn referenced_size(link: &Path, visited: &mut HashSet<PathBuf>) -> u64 {
    referenced_stats(link, visited).await.symlink
}

async fn direct_entry_count(path: &Path) -> u64 {
    let mut entries = match fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    let mut count = 0;
    while let Ok(Some(_)) = entries.next_entry().await {
        count += 1;
    }
    count
}

#[async_recursion]
async fn recursive_file_count(path: &Path) -> u64 {
    let mut entries = match fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    let mut count = 0;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let child = entry.path();
        let meta = match fs::symlink_metadata(&child).await {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        if crate::util::io::is_symlink_or_reparse(&meta) {
            continue;
        }
        if meta.is_dir() {
            count += recursive_file_count(&child).await;
        } else if meta.is_file() {
            count += 1;
        }
    }
    count
}

fn build_folder_node(
    id: &str,
    node_type: StorageNodeType,
    size: StorageSize,
    count: Option<u64>,
    instance_id: Option<&str>,
    paths: Vec<StoragePath>,
) -> StorageNode {
    StorageNode {
        id: id.to_string(),
        node_type,
        name: None,
        instance_id: instance_id.map(str::to_string),
        size,
        count,
        paths,
        children: None,
    }
}

async fn dir_node(
    id: &str,
    node_type: StorageNodeType,
    dir: &Path,
    count: Option<u64>,
    instance_id: Option<&str>,
) -> Option<StorageNode> {
    let mut visited = HashSet::new();
    let size = to_size(scan_path(dir, ScanMode::Host, &mut visited).await);
    if size.total() == 0 {
        return None;
    }
    Some(build_folder_node(
        id,
        node_type,
        size,
        count,
        instance_id,
        vec![StoragePath {
            path: node_path(dir),
            kind: StoragePathKind::Directory,
        }],
    ))
}

async fn combined_dir_node(
    id: &str,
    node_type: StorageNodeType,
    dirs: &[&Path],
    instance_id: Option<&str>,
) -> Option<StorageNode> {
    let mut size = StorageSize::default();
    for dir in dirs {
        let mut visited = HashSet::new();
        size += to_size(scan_path(dir, ScanMode::Host, &mut visited).await);
    }
    if size.total() == 0 {
        return None;
    }
    Some(build_folder_node(
        id,
        node_type,
        size,
        None,
        instance_id,
        dirs.iter()
            .map(|dir| StoragePath {
                path: node_path(dir),
                kind: StoragePathKind::Directory,
            })
            .collect(),
    ))
}

async fn scan_saves(
    instance_path: &Path,
    instance_id: &str,
) -> Option<StorageNode> {
    let saves_path = instance_path.join("saves");
    let mut visited = HashSet::new();
    let total =
        to_size(scan_path(&saves_path, ScanMode::Host, &mut visited).await);
    if total.total() == 0 {
        return None;
    }

    let mut children = Vec::new();
    let mut worlds_size = StorageSize::default();
    let mut entries = fs::read_dir(&saves_path).await.ok()?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let world_path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&world_path).await else {
            continue;
        };
        if !meta.is_dir() || crate::util::io::is_symlink_or_reparse(&meta) {
            continue;
        }

        let mut world_visited = HashSet::new();
        let world_size = to_size(
            scan_path(&world_path, ScanMode::Host, &mut world_visited).await,
        );
        if world_size.total() == 0 {
            continue;
        }

        let world_name = entry.file_name().to_string_lossy().into_owned();
        worlds_size += world_size;
        children.push(StorageNode {
            id: path_id(&saves_path, &format!("world-{world_name}")),
            node_type: StorageNodeType::World,
            name: Some(world_name.clone()),
            instance_id: Some(instance_id.to_string()),
            size: world_size,
            count: None,
            paths: vec![StoragePath {
                path: node_path(&world_path),
                kind: StoragePathKind::Directory,
            }],
            children: None,
        });
    }

    let other = total - worlds_size;
    if other.total() > 0 {
        children.push(build_folder_node(
            "saves-other",
            StorageNodeType::Other,
            other,
            None,
            Some(instance_id),
            vec![StoragePath {
                path: node_path(&saves_path),
                kind: StoragePathKind::Directory,
            }],
        ));
    }

    Some(StorageNode {
        id: path_id(&saves_path, "saves"),
        node_type: StorageNodeType::Saves,
        name: None,
        instance_id: Some(instance_id.to_string()),
        size: total,
        count: Some(
            children
                .iter()
                .filter(|n| n.node_type == StorageNodeType::World)
                .count() as u64,
        ),
        paths: vec![StoragePath {
            path: node_path(&saves_path),
            kind: StoragePathKind::Directory,
        }],
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    })
}

async fn scan_instance_children(
    instance_path: &Path,
    instance_id: &str,
) -> (Vec<StorageNode>, StorageSize) {
    let mut children = Vec::new();
    let mut covered = StorageSize::default();

    if let Some(node) = dir_node(
        &path_id(instance_path, "mods"),
        StorageNodeType::Mods,
        &instance_path.join("mods"),
        Some(direct_entry_count(&instance_path.join("mods")).await),
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = combined_dir_node(
        &path_id(instance_path, "replay"),
        StorageNodeType::Replay,
        &[
            &instance_path.join("flashback"),
            &instance_path.join("replay_recordings"),
        ],
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = dir_node(
        &path_id(instance_path, "resourcepacks"),
        StorageNodeType::Resourcepacks,
        &instance_path.join("resourcepacks"),
        Some(direct_entry_count(&instance_path.join("resourcepacks")).await),
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = scan_saves(instance_path, instance_id).await {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = dir_node(
        &path_id(instance_path, "schematics"),
        StorageNodeType::Schematics,
        &instance_path.join("schematics"),
        Some(recursive_file_count(&instance_path.join("schematics")).await),
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = dir_node(
        &path_id(instance_path, "screenshots"),
        StorageNodeType::Screenshots,
        &instance_path.join("screenshots"),
        Some(recursive_file_count(&instance_path.join("screenshots")).await),
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = dir_node(
        &path_id(instance_path, "shaderpacks"),
        StorageNodeType::Shaderpacks,
        &instance_path.join("shaderpacks"),
        Some(direct_entry_count(&instance_path.join("shaderpacks")).await),
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = combined_dir_node(
        &path_id(instance_path, "minimap"),
        StorageNodeType::Minimap,
        &[
            &instance_path.join("voxelmap"),
            &instance_path.join("xaero"),
            &instance_path.join("XaeroWaypoints_BACKUP"),
        ],
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    if let Some(node) = combined_dir_node(
        &path_id(instance_path, "distant-horizons"),
        StorageNodeType::DistantHorizons,
        &[
            &instance_path.join(".voxy"),
            &instance_path.join("Distant_Horizons_server_data"),
        ],
        Some(instance_id),
    )
    .await
    {
        covered += node.size;
        children.push(node);
    }

    (children, covered)
}

async fn scan_instance(
    instance: &InstanceMetadata,
) -> crate::Result<Option<StorageNode>> {
    let directories = dirs()?;
    let instance_path =
        directories.instances_dir().join(&instance.instance.path);

    let mut visited = HashSet::new();
    let total = match fs::symlink_metadata(&instance_path).await {
        Ok(meta) if crate::util::io::is_symlink_or_reparse(&meta) => {
            to_size(referenced_stats(&instance_path, &mut visited).await)
        }
        _ => to_size(
            scan_path(&instance_path, ScanMode::Host, &mut visited).await,
        ),
    };
    if total.total() == 0 {
        return Ok(None);
    }

    let (mut children, covered) =
        scan_instance_children(&instance_path, &instance.instance.id).await;

    let other = total - covered;
    if other.total() > 0 {
        children.push(build_folder_node(
            "instance-other",
            StorageNodeType::Other,
            other,
            None,
            Some(&instance.instance.id),
            vec![StoragePath {
                path: node_path(&instance_path),
                kind: StoragePathKind::Directory,
            }],
        ));
    }

    Ok(Some(StorageNode {
        id: format!("instance-{}", instance.instance.id),
        node_type: StorageNodeType::Instance,
        name: Some(instance.instance.name.clone()),
        instance_id: Some(instance.instance.id.clone()),
        size: total,
        count: None,
        paths: vec![StoragePath {
            path: node_path(&instance_path),
            kind: StoragePathKind::Directory,
        }],
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }))
}

pub async fn scan_instances_category() -> crate::Result<Option<StorageNode>> {
    let directories = dirs()?;
    let instances_dir = directories.instances_dir();

    let mut visited = HashSet::new();
    let root_total =
        to_size(scan_path(&instances_dir, ScanMode::Host, &mut visited).await);

    let mut children = Vec::new();
    let mut covered = StorageSize::default();

    for instance in crate::api::instance::list().await? {
        if let Some(node) = scan_instance(&instance).await? {
            covered += node.size;
            children.push(node);
        }
    }

    let other = root_total - covered;
    if other.total() > 0 {
        children.push(build_folder_node(
            "profiles-root-other",
            StorageNodeType::Other,
            other,
            None,
            None,
            vec![StoragePath {
                path: node_path(&instances_dir),
                kind: StoragePathKind::Directory,
            }],
        ));
    }

    let total = root_total;
    if total.total() == 0 {
        return Ok(None);
    }

    Ok(Some(StorageNode {
        id: "category-instances".to_string(),
        node_type: StorageNodeType::Instances,
        name: None,
        instance_id: None,
        size: total,
        count: Some(children.len() as u64),
        paths: vec![StoragePath {
            path: node_path(&instances_dir),
            kind: StoragePathKind::Directory,
        }],
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }))
}

pub async fn scan_cache_category() -> crate::Result<Option<StorageNode>> {
    let directories = dirs()?;
    Ok(dir_node(
        "category-cache",
        StorageNodeType::Cache,
        &directories.caches_dir(),
        None,
        None,
    )
    .await)
}

pub async fn scan_meta_category() -> crate::Result<Option<StorageNode>> {
    let directories = dirs()?;
    Ok(dir_node(
        "category-meta",
        StorageNodeType::Meta,
        &directories.metadata_dir(),
        None,
        None,
    )
    .await)
}

pub async fn scan_database_category() -> crate::Result<Option<StorageNode>> {
    let directories = dirs()?;
    let settings_dir = directories.settings_dir.clone();

    let app_db = settings_dir.join("app.db");
    let app_db_size = fs::metadata(&app_db).await.map(|m| m.len()).unwrap_or(0);

    let mut total = StorageSize {
        actual: app_db_size,
        symlink: 0,
    };
    let mut children = Vec::new();

    if app_db_size > 0 {
        children.push(StorageNode {
            id: "database-app-db".to_string(),
            node_type: StorageNodeType::DbFile,
            name: None,
            instance_id: None,
            size: StorageSize {
                actual: app_db_size,
                symlink: 0,
            },
            count: None,
            paths: vec![StoragePath {
                path: node_path(&app_db),
                kind: StoragePathKind::File,
            }],
            children: None,
        });
    }

    let backups_dir = settings_dir.join("Backups").join("app-db");
    let mut visited = HashSet::new();
    let backups_size =
        to_size(scan_path(&backups_dir, ScanMode::Host, &mut visited).await);
    total += backups_size;

    if backups_size.total() > 0 {
        children.push(StorageNode {
            id: "database-backups".to_string(),
            node_type: StorageNodeType::DbBackup,
            name: None,
            instance_id: None,
            size: backups_size,
            count: None,
            paths: vec![StoragePath {
                path: node_path(&backups_dir),
                kind: StoragePathKind::Directory,
            }],
            children: None,
        });
    }

    let mut sidecar_total = 0u64;
    for suffix in ["-wal", "-shm"] {
        let path = settings_dir.join(format!("app.db{suffix}"));
        sidecar_total += fs::metadata(path).await.map(|m| m.len()).unwrap_or(0);
    }
    total.actual = total.actual.saturating_add(sidecar_total);

    let covered = children
        .iter()
        .fold(StorageSize::default(), |acc, child| acc + child.size);
    let other = total - covered;
    if other.total() > 0 {
        children.push(build_folder_node(
            "database-other",
            StorageNodeType::Other,
            other,
            None,
            None,
            vec![StoragePath {
                path: node_path(&settings_dir),
                kind: StoragePathKind::Directory,
            }],
        ));
    }

    if total.total() == 0 {
        return Ok(None);
    }

    Ok(Some(StorageNode {
        id: "category-database".to_string(),
        node_type: StorageNodeType::Database,
        name: None,
        instance_id: None,
        size: total,
        count: Some(children.len() as u64),
        paths: vec![StoragePath {
            path: node_path(&settings_dir),
            kind: StoragePathKind::Directory,
        }],
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }))
}

pub async fn scan_root_other(
    known: StorageSize,
) -> crate::Result<Option<StorageNode>> {
    let directories = dirs()?;
    let settings_dir = directories.settings_dir.clone();
    let config_dir = directories.config_dir.clone();

    let mut visited = HashSet::new();
    let mut root_total =
        to_size(scan_path(&settings_dir, ScanMode::Host, &mut visited).await);

    if settings_dir != config_dir {
        let mut config_visited = HashSet::new();
        root_total += to_size(
            scan_path(&config_dir, ScanMode::Host, &mut config_visited).await,
        );
    }

    let other = root_total - known;
    if other.total() == 0 {
        return Ok(None);
    }

    let mut paths = vec![StoragePath {
        path: node_path(&settings_dir),
        kind: StoragePathKind::Directory,
    }];
    if settings_dir != config_dir {
        paths.push(StoragePath {
            path: node_path(&config_dir),
            kind: StoragePathKind::Directory,
        });
    }

    Ok(Some(build_folder_node(
        "root-other",
        StorageNodeType::Other,
        other,
        None,
        None,
        paths,
    )))
}

pub fn assemble_storage_tree(
    categories: Vec<StorageNode>,
    root_other: Option<StorageNode>,
) -> StorageTree {
    let mut total = StorageSize::default();
    for category in &categories {
        total += category.size;
    }
    if let Some(other) = &root_other {
        total += other.size;
    }

    StorageTree {
        version: STORAGE_CACHE_VERSION,
        scanned_at: Utc::now(),
        total,
        categories,
        root_other,
    }
}

pub async fn scan_storage_tree() -> crate::Result<StorageTree> {
    let mut categories = Vec::new();
    let mut known = StorageSize::default();

    if let Some(node) = scan_instances_category().await? {
        known += node.size;
        categories.push(node);
    }
    if let Some(node) = scan_cache_category().await? {
        known += node.size;
        categories.push(node);
    }
    if let Some(node) = scan_meta_category().await? {
        known += node.size;
        categories.push(node);
    }
    if let Some(node) = scan_database_category().await? {
        known += node.size;
        categories.push(node);
    }

    let root_other = scan_root_other(known).await?;
    Ok(assemble_storage_tree(categories, root_other))
}

fn storage_cache_path(directories: &DirectoryInfo) -> PathBuf {
    directories.caches_dir().join(STORAGE_CACHE_FILE)
}

fn roots_signature(directories: &DirectoryInfo) -> String {
    format!(
        "{}\u{1f}{}",
        directories.settings_dir.display(),
        directories.config_dir.display()
    )
}

pub async fn load_storage_cache() -> Option<StorageTree> {
    let directories = dirs().ok()?;
    let path = storage_cache_path(&directories);
    let bytes = fs::read(path).await.ok()?;

    let cache: StorageCacheFile = serde_cbor::from_slice(&bytes).ok()?;
    if cache.version != STORAGE_CACHE_VERSION {
        return None;
    }
    if cache.roots != roots_signature(&directories) {
        return None;
    }

    Some(cache.tree)
}

pub async fn save_storage_cache(tree: &StorageTree) -> crate::Result<()> {
    let directories = dirs()?;
    let path = storage_cache_path(&directories);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let cache = StorageCacheFile {
        version: STORAGE_CACHE_VERSION,
        roots: roots_signature(&directories),
        scanned_at: tree.scanned_at,
        tree: tree.clone(),
    };
    let bytes = serde_cbor::to_vec(&cache).map_err(|error| {
        crate::Error::from(crate::ErrorKind::FSError(format!(
            "Failed to serialize storage cache: {error}"
        )))
    })?;

    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, &bytes).await?;
    fs::rename(&temp_path, &path).await?;
    Ok(())
}
