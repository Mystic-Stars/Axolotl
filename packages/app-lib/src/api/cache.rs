use crate::state::{
    CacheBehaviour, CacheValueType, CachedEntry, ModrinthProjectId,
    ModrinthVersionId, Organization, Project, ProjectV3, SearchResults,
    SearchResultsV3, TeamMember, User, Version,
};

macro_rules! impl_cache_methods {
    ($(($variant:ident, $type:ty)),*) => {
        $(
            paste::paste! {
                #[tracing::instrument]
                pub async fn [<get_ $variant:snake>](
                    id: &str,
                    cache_behaviour: Option<CacheBehaviour>,
                ) -> crate::Result<Option<$type>>
                {
                    let state = crate::State::get().await?;
                    Ok(CachedEntry::[<get_ $variant:snake _many>](&[id], cache_behaviour, &state.pool, &state.api_semaphore).await?.into_iter().next())
                }

                #[tracing::instrument]
                pub async fn [<get_ $variant:snake _many>](
                    ids: &[&str],
                    cache_behaviour: Option<CacheBehaviour>,
                ) -> crate::Result<Vec<$type>>
                {
                    let state = crate::State::get().await?;
                    let entries =
                        CachedEntry::[<get_ $variant:snake _many>](ids, None, &state.pool, &state.api_semaphore).await?;

                    Ok(entries)
                }
            }
        )*
    }
}

impl_cache_methods!(
    (ProjectV3, ProjectV3),
    (User, User),
    (Team, Vec<TeamMember>),
    (Organization, Organization),
    (SearchResults, SearchResults),
    (SearchResultsV3, SearchResultsV3)
);

#[tracing::instrument]
pub async fn get_project(
    id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Option<Project>> {
    let id = ModrinthProjectId::new(id.to_string())?;
    let state = crate::State::get().await?;
    CachedEntry::get_project(
        &id,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}

#[tracing::instrument]
pub async fn get_project_many(
    ids: &[&str],
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Vec<Project>> {
    let ids = ids
        .iter()
        .map(|id| ModrinthProjectId::new((*id).to_string()))
        .collect::<crate::Result<Vec<_>>>()?;
    let state = crate::State::get().await?;
    CachedEntry::get_project_many(
        &ids,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}

#[tracing::instrument]
pub async fn get_version(
    id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Option<Version>> {
    let id = ModrinthVersionId::new(id.to_string())?;
    let state = crate::State::get().await?;
    CachedEntry::get_version(
        &id,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}

#[tracing::instrument]
pub async fn get_version_many(
    ids: &[&str],
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Vec<Version>> {
    let ids = ids
        .iter()
        .map(|id| ModrinthVersionId::new((*id).to_string()))
        .collect::<crate::Result<Vec<_>>>()?;
    let state = crate::State::get().await?;
    CachedEntry::get_version_many(
        &ids,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}

pub async fn purge_cache_types(
    cache_types: &[CacheValueType],
) -> crate::Result<()> {
    let state = crate::State::get().await?;
    CachedEntry::purge_cache_types(cache_types, &state.pool).await?;

    Ok(())
}

/// Get versions for a project (without changelogs for fast loading).
/// Uses the cache system with the ProjectVersions cache type.
#[tracing::instrument]
pub async fn get_project_versions(
    project_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Option<Vec<Version>>> {
    let project_id = ModrinthProjectId::new(project_id.to_string())?;
    let state = crate::State::get().await?;
    CachedEntry::get_project_versions(
        &project_id,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}
