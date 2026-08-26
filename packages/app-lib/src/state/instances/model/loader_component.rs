use crate::state::ModLoader;
use serde::{Deserialize, Serialize};

use super::unknown_value;

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
#[serde(rename_all = "snake_case")]
pub enum LoaderComponentKind {
    Vanilla,
    Forge,
    #[serde(rename = "neoforge", alias = "neo_forge")]
    NeoForge,
    Fabric,
    Quilt,
    Cleanroom,
    LegacyFabric,
    Babric,
    #[serde(rename = "optifine", alias = "opti_fine")]
    OptiFine,
    LiteLoader,
    #[serde(rename = "optifabric", alias = "opti_fabric")]
    OptiFabric,
}

impl LoaderComponentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vanilla => "vanilla",
            Self::Forge => "forge",
            Self::NeoForge => "neoforge",
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::Cleanroom => "cleanroom",
            Self::LegacyFabric => "legacy_fabric",
            Self::Babric => "babric",
            Self::OptiFine => "optifine",
            Self::LiteLoader => "lite_loader",
            Self::OptiFabric => "optifabric",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "vanilla" => Ok(Self::Vanilla),
            "forge" => Ok(Self::Forge),
            "neoforge" => Ok(Self::NeoForge),
            "fabric" => Ok(Self::Fabric),
            "quilt" => Ok(Self::Quilt),
            "cleanroom" => Ok(Self::Cleanroom),
            "legacy_fabric" => Ok(Self::LegacyFabric),
            "babric" => Ok(Self::Babric),
            "optifine" => Ok(Self::OptiFine),
            "lite_loader" => Ok(Self::LiteLoader),
            "optifabric" => Ok(Self::OptiFabric),
            other => Err(unknown_value("loader component kind", other)),
        }
    }

    pub fn from_loader(loader: ModLoader) -> Self {
        match loader {
            ModLoader::Vanilla => Self::Vanilla,
            ModLoader::Forge => Self::Forge,
            ModLoader::NeoForge => Self::NeoForge,
            ModLoader::Fabric => Self::Fabric,
            ModLoader::Quilt => Self::Quilt,
            ModLoader::Cleanroom => Self::Cleanroom,
            ModLoader::LegacyFabric => Self::LegacyFabric,
            ModLoader::Babric => Self::Babric,
            ModLoader::OptiFine => Self::OptiFine,
            ModLoader::LiteLoader => Self::LiteLoader,
        }
    }

    pub fn as_loader(self) -> Option<ModLoader> {
        match self {
            Self::Vanilla => Some(ModLoader::Vanilla),
            Self::Forge => Some(ModLoader::Forge),
            Self::NeoForge => Some(ModLoader::NeoForge),
            Self::Fabric => Some(ModLoader::Fabric),
            Self::Quilt => Some(ModLoader::Quilt),
            Self::Cleanroom => Some(ModLoader::Cleanroom),
            Self::LegacyFabric => Some(ModLoader::LegacyFabric),
            Self::Babric => Some(ModLoader::Babric),
            Self::OptiFine => Some(ModLoader::OptiFine),
            Self::LiteLoader => Some(ModLoader::LiteLoader),
            Self::OptiFabric => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoaderComponentRole {
    Primary,
    Adjunct,
}

impl LoaderComponentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Adjunct => "adjunct",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "primary" => Ok(Self::Primary),
            "adjunct" => Ok(Self::Adjunct),
            other => Err(unknown_value("loader component role", other)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderComponent {
    pub instance_id: String,
    pub kind: LoaderComponentKind,
    pub version: Option<String>,
    pub role: LoaderComponentRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
}

impl LoaderComponent {
    pub fn new_primary(
        instance_id: impl Into<String>,
        loader: ModLoader,
        version: Option<String>,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            kind: LoaderComponentKind::from_loader(loader),
            version,
            role: LoaderComponentRole::Primary,
            provider_metadata: None,
        }
    }

    pub fn from_legacy_projection(
        instance_id: impl Into<String>,
        loader: ModLoader,
        version: Option<String>,
    ) -> Vec<Self> {
        let instance_id = instance_id.into();
        match loader {
            ModLoader::OptiFine | ModLoader::LiteLoader => vec![
                Self::new_primary(
                    instance_id.clone(),
                    ModLoader::Vanilla,
                    None,
                ),
                Self {
                    instance_id,
                    kind: LoaderComponentKind::from_loader(loader),
                    version,
                    role: LoaderComponentRole::Adjunct,
                    provider_metadata: None,
                },
            ],
            _ => vec![Self::new_primary(instance_id, loader, version)],
        }
    }
}

pub fn project_loader_components(
    components: &[LoaderComponent],
) -> crate::Result<(ModLoader, Option<String>)> {
    let primary = components
        .iter()
        .find(|component| component.role == LoaderComponentRole::Primary)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "Loader component set has no primary component".to_string(),
            )
            .as_error()
        })?;
    let primary_loader = primary.kind.as_loader().ok_or_else(|| {
        crate::ErrorKind::InputError(
            "OptiFabric cannot be a primary loader component".to_string(),
        )
        .as_error()
    })?;

    if primary_loader != ModLoader::Vanilla {
        return Ok((primary_loader, primary.version.clone()));
    }

    for kind in [
        LoaderComponentKind::OptiFine,
        LoaderComponentKind::LiteLoader,
    ] {
        if let Some(adjunct) = components.iter().find(|component| {
            component.role == LoaderComponentRole::Adjunct
                && component.kind == kind
        }) {
            return Ok((
                kind.as_loader().expect("projectable adjunct"),
                adjunct.version.clone(),
            ));
        }
    }

    Ok((ModLoader::Vanilla, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_primary_wins_over_optifine_adjunct() {
        let components = vec![
            LoaderComponent::new_primary(
                "instance",
                ModLoader::Forge,
                Some("47.4.0".to_string()),
            ),
            LoaderComponent {
                instance_id: "instance".to_string(),
                kind: LoaderComponentKind::OptiFine,
                version: Some("HD_U_I6".to_string()),
                role: LoaderComponentRole::Adjunct,
                provider_metadata: None,
            },
        ];

        assert_eq!(
            project_loader_components(&components).unwrap(),
            (ModLoader::Forge, Some("47.4.0".to_string()))
        );
    }

    #[test]
    fn vanilla_optifine_preserves_legacy_projection() {
        let components = LoaderComponent::from_legacy_projection(
            "instance",
            ModLoader::OptiFine,
            Some("HD_U_I6".to_string()),
        );

        assert_eq!(
            project_loader_components(&components).unwrap(),
            (ModLoader::OptiFine, Some("HD_U_I6".to_string()))
        );
    }

    #[test]
    fn loader_wire_names_are_canonical_and_accept_legacy_aliases() {
        for (loader, canonical, legacy_alias) in [
            (ModLoader::NeoForge, "neoforge", "neo_forge"),
            (ModLoader::OptiFine, "optifine", "opti_fine"),
        ] {
            assert_eq!(
                serde_json::to_string(&loader).unwrap(),
                format!("\"{canonical}\"")
            );
            assert_eq!(
                serde_json::from_str::<ModLoader>(&format!("\"{canonical}\""))
                    .unwrap(),
                loader
            );
            assert_eq!(
                serde_json::from_str::<ModLoader>(&format!(
                    "\"{legacy_alias}\""
                ))
                .unwrap(),
                loader
            );
        }

        for (kind, canonical, legacy_alias) in [
            (LoaderComponentKind::NeoForge, "neoforge", "neo_forge"),
            (LoaderComponentKind::OptiFine, "optifine", "opti_fine"),
            (LoaderComponentKind::OptiFabric, "optifabric", "opti_fabric"),
        ] {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{canonical}\"")
            );
            assert_eq!(
                serde_json::from_str::<LoaderComponentKind>(&format!(
                    "\"{legacy_alias}\""
                ))
                .unwrap(),
                kind
            );
        }
    }
}
