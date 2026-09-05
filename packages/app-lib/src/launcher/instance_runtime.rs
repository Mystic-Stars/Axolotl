use std::path::{Path, PathBuf};

use crate::state::{DirectoryInfo, Instance};

use super::DirectLinkedLaunch;

/// Runtime storage adapters keep mode-specific path rules out of callers.
#[derive(Debug, Clone)]
pub(crate) enum InstanceRuntimeAdapter {
    AxolotlManaged { game_dir: PathBuf },
    MinecraftShared { direct: DirectLinkedLaunch },
    MinecraftIsolated { direct: DirectLinkedLaunch },
}

impl InstanceRuntimeAdapter {
    /// Entry point selecting the adapter for an instance.
    pub(crate) fn for_instance(
        instance: &Instance,
        directories: &DirectoryInfo,
    ) -> crate::Result<Self> {
        if let Some(external) = Self::external_for_instance(instance)? {
            return Ok(external);
        }

        Ok(Self::AxolotlManaged {
            game_dir: directories.instance_game_dir(instance),
        })
    }

    pub(crate) fn external_for_instance(
        instance: &Instance,
    ) -> crate::Result<Option<Self>> {
        if let Some(direct) = DirectLinkedLaunch::from_instance(instance)? {
            return Ok(Some(Self::from_direct(direct)));
        }

        let Some(path) = instance.game_dir_override.as_deref() else {
            return Ok(None);
        };
        let Some(direct) =
            DirectLinkedLaunch::from_external_version_dir(Path::new(path))?
        else {
            return Ok(None);
        };
        Ok(Some(Self::MinecraftIsolated { direct }))
    }

    fn from_direct(direct: DirectLinkedLaunch) -> Self {
        let isolated = match direct.dialect {
            super::LinkedLauncherDialect::Pcl
            | super::LinkedLauncherDialect::PclCe => true,
            super::LinkedLauncherDialect::Generic => true,
            super::LinkedLauncherDialect::Hmcl => false,
        };
        if isolated {
            Self::MinecraftIsolated { direct }
        } else {
            Self::MinecraftShared { direct }
        }
    }

    pub(crate) fn direct_link(&self) -> Option<&DirectLinkedLaunch> {
        match self {
            Self::AxolotlManaged { .. } => None,
            Self::MinecraftShared { direct }
            | Self::MinecraftIsolated { direct } => Some(direct),
        }
    }

    pub(crate) fn game_dir(&self) -> PathBuf {
        match self {
            Self::AxolotlManaged { game_dir } => game_dir.clone(),
            Self::MinecraftShared { direct } => match direct.resolve() {
                Ok(resolved) => resolved.game_dir,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "Falling back after the external instance version chain could not be resolved"
                    );
                    direct.dot_minecraft.clone()
                }
            },
            Self::MinecraftIsolated { direct } => match direct.resolve() {
                Ok(resolved) => resolved.game_dir,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "Falling back to the external instance version directory"
                    );
                    direct.version_dir()
                }
            },
        }
    }

    pub(crate) fn libraries_dir(&self, directories: &DirectoryInfo) -> PathBuf {
        self.direct_link().map_or_else(
            || directories.libraries_dir(),
            DirectLinkedLaunch::libraries_dir,
        )
    }

    pub(crate) fn assets_dir(&self, directories: &DirectoryInfo) -> PathBuf {
        self.direct_link().map_or_else(
            || directories.assets_dir(),
            DirectLinkedLaunch::assets_dir,
        )
    }

    pub(crate) fn log_configs_dir(
        &self,
        directories: &DirectoryInfo,
    ) -> PathBuf {
        self.direct_link().map_or_else(
            || directories.log_configs_dir(),
            DirectLinkedLaunch::log_configs_dir,
        )
    }
}
