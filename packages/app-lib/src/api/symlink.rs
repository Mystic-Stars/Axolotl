use crate::util::symlink::SymlinkCapability;

#[tracing::instrument]
pub async fn check_symlink_capability() -> crate::Result<SymlinkCapability> {
    Ok(crate::util::symlink::check_symlink_capability().await)
}

#[tracing::instrument]
pub async fn requires_admin() -> bool {
    matches!(
        crate::util::symlink::check_symlink_capability().await,
        crate::util::symlink::SymlinkCapability::RequiresAdmin
    )
}