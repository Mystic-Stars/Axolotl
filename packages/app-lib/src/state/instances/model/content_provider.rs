use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

use super::unknown_value;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentProvider {
    Modrinth,
    #[serde(rename = "curseforge")]
    CurseForge,
}

impl ContentProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::CurseForge => "curseforge",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "modrinth" => Ok(Self::Modrinth),
            "curseforge" => Ok(Self::CurseForge),
            other => Err(unknown_value("content provider", other)),
        }
    }
}

macro_rules! modrinth_id {
    ($name:ident, $kind:literal) => {
        #[derive(
            Clone,
            Debug,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            Serialize,
            Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> crate::Result<Self> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(crate::ErrorKind::InputError(format!(
                        "Empty {}",
                        $kind
                    ))
                    .into());
                }

                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

modrinth_id!(ModrinthProjectId, "Modrinth project ID");
modrinth_id!(ModrinthVersionId, "Modrinth version ID");

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct CurseForgeProjectId(u32);

impl CurseForgeProjectId {
    pub fn new(value: u32) -> crate::Result<Self> {
        if value == 0 {
            return Err(crate::ErrorKind::InputError(
                "Invalid CurseForge project ID 0".to_string(),
            )
            .into());
        }

        Ok(Self(value))
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct CurseForgeFileId(u32);

impl CurseForgeFileId {
    pub fn new(value: u32) -> crate::Result<Self> {
        if value == 0 {
            return Err(crate::ErrorKind::InputError(
                "Invalid CurseForge file ID 0".to_string(),
            )
            .into());
        }

        Ok(Self(value))
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum ContentProviderRef {
    Modrinth {
        project_id: ModrinthProjectId,
        version_id: Option<ModrinthVersionId>,
    },
    CurseForge {
        project_id: CurseForgeProjectId,
        file_id: Option<CurseForgeFileId>,
    },
}

impl ContentProviderRef {
    pub fn provider(&self) -> ContentProvider {
        match self {
            Self::Modrinth { .. } => ContentProvider::Modrinth,
            Self::CurseForge { .. } => ContentProvider::CurseForge,
        }
    }

    pub fn from_database(
        provider: &str,
        project_id: &str,
        release_id: Option<&str>,
    ) -> crate::Result<Self> {
        match ContentProvider::from_str(provider)? {
            ContentProvider::Modrinth => Ok(Self::Modrinth {
                project_id: ModrinthProjectId::new(project_id)?,
                version_id: release_id
                    .map(ModrinthVersionId::new)
                    .transpose()?,
            }),
            ContentProvider::CurseForge => Ok(Self::CurseForge {
                project_id: CurseForgeProjectId::new(
                    project_id.parse().map_err(|_| {
                        crate::ErrorKind::InputError(format!(
                            "Invalid CurseForge project ID {project_id}"
                        ))
                    })?,
                )?,
                file_id: match release_id {
                    Some(value) => Some(CurseForgeFileId::new(
                        value.parse().map_err(|_| {
                            crate::ErrorKind::InputError(format!(
                                "Invalid CurseForge file ID {value}"
                            ))
                        })?,
                    )?),
                    None => None,
                },
            }),
        }
    }

    pub fn database_project_id(&self) -> String {
        match self {
            Self::Modrinth { project_id, .. } => project_id.to_string(),
            Self::CurseForge { project_id, .. } => project_id.get().to_string(),
        }
    }

    pub fn database_release_id(&self) -> Option<String> {
        match self {
            Self::Modrinth { version_id, .. } => {
                version_id.as_ref().map(ToString::to_string)
            }
            Self::CurseForge { file_id, .. } => {
                file_id.map(|value| value.get().to_string())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum ContentItemUpdate {
    Modrinth {
        project_id: ModrinthProjectId,
        current_version_id: ModrinthVersionId,
        target_version_id: ModrinthVersionId,
    },
    CurseForge {
        project_id: CurseForgeProjectId,
        current_file_id: CurseForgeFileId,
        target_file_id: CurseForgeFileId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_cannot_be_cross_parsed() {
        let curseforge_project = CurseForgeProjectId::new(42).unwrap();
        let curseforge_file = CurseForgeFileId::new(7).unwrap();
        assert!(
            ModrinthProjectId::new(curseforge_project.get().to_string())
                .is_ok()
        );
        assert!(
            ModrinthVersionId::new(curseforge_file.get().to_string()).is_ok()
        );

        let reference = ContentProviderRef::CurseForge {
            project_id: curseforge_project,
            file_id: Some(curseforge_file),
        };
        assert!(matches!(reference, ContentProviderRef::CurseForge { .. }));
        assert!(!matches!(reference, ContentProviderRef::Modrinth { .. }));
    }

    #[test]
    fn malformed_database_references_are_rejected() {
        assert!(
            ContentProviderRef::from_database(
                "curseforge",
                "not-a-number",
                Some("7"),
            )
            .is_err()
        );
        assert!(
            ContentProviderRef::from_database("unknown", "project", None,)
                .is_err()
        );
    }

    #[test]
    fn same_numeric_id_keeps_provider_identity() {
        let modrinth = ContentProviderRef::Modrinth {
            project_id: ModrinthProjectId::new("42").unwrap(),
            version_id: Some(ModrinthVersionId::new("7").unwrap()),
        };
        let curseforge = ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(42).unwrap(),
            file_id: Some(CurseForgeFileId::new(7).unwrap()),
        };
        assert_ne!(modrinth, curseforge);
        assert_ne!(modrinth.provider(), curseforge.provider());
    }
}
