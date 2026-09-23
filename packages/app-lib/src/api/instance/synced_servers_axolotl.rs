use crate::State;
use crate::state::InstanceMetadata;
use quartz_nbt::{NbtCompound, NbtList, NbtTag};
use std::path::PathBuf;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncedServer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub accept_textures: Option<bool>,
}

pub(crate) async fn merge_servers_from_instance(
    metadata: &InstanceMetadata,
    state: &State,
) -> crate::Result<()> {
    if !metadata.synced_options.multiplayer_servers {
        return Ok(());
    }
    reconcile_servers(metadata, state).await
}
pub(crate) async fn reconcile_servers(
    metadata: &InstanceMetadata,
    state: &State,
) -> crate::Result<()> {
    if !metadata.synced_options.multiplayer_servers {
        return Ok(());
    }
    let local =
        crate::api::instance::synced_options::instance_dir(metadata, state)
            .join("servers.dat");
    if !local.exists() {
        return Ok(());
    }
    let bytes = tokio::fs::read(&local).await?;
    let canonical = canonical_path(state);
    if canonical.exists() && tokio::fs::read(&canonical).await? == bytes {
        return Ok(());
    }
    let servers = decode_servers(&bytes)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM synced_servers")
        .execute(&mut *tx)
        .await?;
    for (position, nbt) in servers.into_iter().enumerate() {
        let id = server_id(&nbt);
        let encoded = encode_server(&nbt)?;
        sqlx::query("INSERT INTO synced_servers(id,position,nbt) VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET position=excluded.position,nbt=excluded.nbt")
            .bind(id).bind(position as i64).bind(encoded).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO synced_server_state(singleton,revision) VALUES(1,1) ON CONFLICT(singleton) DO UPDATE SET revision=revision+1")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    write_and_project(state).await?;
    Ok(())
}
pub(crate) async fn seed_servers(
    metadata: &InstanceMetadata,
    state: &State,
) -> crate::Result<()> {
    reconcile_servers(metadata, state).await
}
pub(crate) async fn ensure_servers(
    metadata: &InstanceMetadata,
    state: &State,
) -> crate::Result<()> {
    let canonical = canonical_path(state);
    if !canonical.exists() {
        return reconcile_servers(metadata, state).await;
    }
    let target =
        crate::api::instance::synced_options::instance_dir(metadata, state)
            .join("servers.dat");
    if !target.exists() {
        tokio::fs::copy(canonical, target).await?;
    }
    Ok(())
}
pub(crate) async fn detach_servers(
    _: &InstanceMetadata,
    _: &State,
) -> crate::Result<()> {
    Ok(())
}
pub(crate) async fn canonical_exists(state: &State) -> crate::Result<bool> {
    Ok(canonical_path(state).exists())
}
pub async fn list_synced_servers() -> crate::Result<Vec<SyncedServer>> {
    let state = State::get().await?;
    let rows = sqlx::query(
        "SELECT id, position, nbt FROM synced_servers ORDER BY position",
    )
    .fetch_all(&state.pool)
    .await?;
    use sqlx::Row;
    rows.into_iter()
        .map(|r| {
            let data = crate::api::instance::synced_options::nbt_from_bytes(
                r.get("nbt"),
            )?;
            Ok(SyncedServer {
                id: r.get("id"),
                name: data
                    .get::<_, &str>("name")
                    .unwrap_or_default()
                    .to_string(),
                address: data
                    .get::<_, &str>("ip")
                    .unwrap_or_default()
                    .to_string(),
                accept_textures: data
                    .get::<_, i8>("acceptTextures")
                    .ok()
                    .map(|value| value != 0),
            })
        })
        .collect()
}
pub async fn update_synced_server(server: SyncedServer) -> crate::Result<()> {
    let state = State::get().await?;
    let mut data = NbtCompound::new();
    data.insert("name", server.name);
    data.insert("ip", server.address);
    data.insert("hidden", 0_i8);
    if let Some(value) = server.accept_textures {
        data.insert("acceptTextures", i8::from(value));
    }
    let nbt = encode_server(&data)?;
    let position: i64=sqlx::query_scalar("SELECT COALESCE((SELECT position FROM synced_servers WHERE id=?),(SELECT COALESCE(MAX(position),-1)+1 FROM synced_servers))").bind(&server.id).fetch_one(&state.pool).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO synced_servers(id,position,nbt) VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET nbt=excluded.nbt").bind(server.id).bind(position).bind(nbt).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO synced_server_state(singleton,revision) VALUES(1,1) ON CONFLICT(singleton) DO UPDATE SET revision=revision+1").execute(&mut *tx).await?;
    tx.commit().await?;
    write_and_project(&state).await?;
    Ok(())
}
pub async fn remove_synced_server(id: String) -> crate::Result<()> {
    let state = State::get().await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM synced_servers WHERE id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO synced_server_state(singleton,revision) VALUES(1,1) ON CONFLICT(singleton) DO UPDATE SET revision=revision+1").execute(&mut *tx).await?;
    tx.commit().await?;
    normalize_positions(&state).await?;
    write_and_project(&state).await?;
    Ok(())
}
fn canonical_path(state: &State) -> PathBuf {
    state.directories.synced_options_dir().join("servers.dat")
}

fn decode_servers(bytes: &[u8]) -> crate::Result<Vec<NbtCompound>> {
    let root =
        crate::api::instance::synced_options::nbt_from_bytes(bytes.to_vec())?;
    let list = root.get::<_, &NbtList>("servers").map_err(|_| {
        crate::ErrorKind::InputError(
            "servers.dat does not contain a valid servers list".to_string(),
        )
    })?;
    list.iter()
        .map(|tag| match tag {
            NbtTag::Compound(value) => Ok(value.clone()),
            _ => Err(crate::ErrorKind::InputError(
                "servers.dat contains an invalid server entry".to_string(),
            )
            .into()),
        })
        .collect()
}

fn encode_server(server: &NbtCompound) -> crate::Result<Vec<u8>> {
    crate::api::instance::synced_options::nbt_to_bytes(server)
}

fn server_id(server: &NbtCompound) -> String {
    let name = server.get::<_, &str>("name").unwrap_or_default();
    let address = server.get::<_, &str>("ip").unwrap_or_default();
    crate::api::instance::synced_options::sha1_bytes(
        format!(
            "{}\0{}",
            name.trim().to_ascii_lowercase(),
            address.trim().to_ascii_lowercase()
        )
        .as_bytes(),
    )
}

async fn write_canonical(state: &State) -> crate::Result<()> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT nbt FROM synced_servers ORDER BY position")
        .fetch_all(&state.pool)
        .await?;
    let mut list = NbtList::new();
    for row in rows {
        let bytes: Vec<u8> = row.get("nbt");
        list.push(crate::api::instance::synced_options::nbt_from_bytes(bytes)?);
    }
    let mut root = NbtCompound::new();
    root.insert("servers", list);
    let path = canonical_path(state);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(
        path,
        crate::api::instance::synced_options::nbt_to_bytes(&root)?,
    )
    .await?;
    Ok(())
}

async fn normalize_positions(state: &State) -> crate::Result<()> {
    use sqlx::Row;
    let rows =
        sqlx::query("SELECT id FROM synced_servers ORDER BY position, id")
            .fetch_all(&state.pool)
            .await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE synced_servers SET position=position+2000000")
        .execute(&mut *tx)
        .await?;
    for (position, row) in rows.into_iter().enumerate() {
        let id: String = row.get("id");
        sqlx::query("UPDATE synced_servers SET position=? WHERE id=?")
            .bind(position as i64)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn write_and_project(state: &State) -> crate::Result<()> {
    use sqlx::Row;
    write_canonical(state).await?;
    let canonical = canonical_path(state);
    let rows = sqlx::query("SELECT instance_id FROM instance_sync_preferences WHERE feature='multiplayer_servers' AND enabled=1").fetch_all(&state.pool).await?;
    for row in rows {
        let id: String = row.get("instance_id");
        if let Some(metadata) =
            crate::state::get_instance(&id, &state.pool).await?
        {
            let target = crate::api::instance::synced_options::instance_dir(
                &metadata, state,
            )
                .join("servers.dat");
            let excluded = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM instance_servers
                 WHERE instance_id = ? AND source = 'local_desynced'
                   AND excluded_synced_server_id IS NOT NULL",
            )
            .bind(&id)
            .fetch_one(&state.pool)
            .await?
                > 0;
            sqlx::query("DELETE FROM instance_server_projection_entries WHERE instance_id = ?")
                .bind(&id)
                .execute(&state.pool)
                .await?;
            let canonical_rows = sqlx::query(
                "SELECT id, nbt, position FROM synced_servers ORDER BY position",
            )
            .fetch_all(&state.pool)
            .await?;
            for canonical_row in canonical_rows {
                sqlx::query(
                    "INSERT INTO instance_server_projection_entries
                     (instance_id, owner, server_id, nbt, position)
                     VALUES (?, 'synced', ?, ?, ?)
                     ON CONFLICT(instance_id, owner, server_id) DO UPDATE SET
                       nbt=excluded.nbt, position=excluded.position",
                )
                .bind(&id)
                .bind(canonical_row.get::<String, _>("id"))
                .bind(canonical_row.get::<Vec<u8>, _>("nbt"))
                .bind(canonical_row.get::<i64, _>("position"))
                .execute(&state.pool)
                .await?;
            }
            if excluded {
                continue;
            }
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&canonical, target).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_identity_is_case_and_whitespace_insensitive() {
        let mut first = NbtCompound::new();
        first.insert("name", " Example ");
        first.insert("ip", "PLAY.EXAMPLE.COM ");
        let mut second = NbtCompound::new();
        second.insert("name", "example");
        second.insert("ip", "play.example.com");
        assert_eq!(server_id(&first), server_id(&second));
    }

    #[test]
    fn server_compound_round_trips_through_storage() {
        let mut server = NbtCompound::new();
        server.insert("name", "Example");
        server.insert("ip", "play.example.com");
        server.insert("acceptTextures", 1_i8);
        let decoded = crate::api::instance::synced_options::nbt_from_bytes(
            encode_server(&server).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.get::<_, &str>("name").unwrap(), "Example");
        assert_eq!(decoded.get::<_, &str>("ip").unwrap(), "play.example.com");
    }
}
