//! Determines how version info is generated for pairs of game and loader
//! versions.
//!
//! When a user installs a version of the game, they install two things: the
//! game (of some specific version), and a loader (of some other specific
//! version). Each combination of game and loader version requires a specific
//! configuration, like a specific set of libraries that must be downloaded and
//! run along with the game. However, some versions of the game or loader may
//! change configuration requirements without the other version being affected.
//! For example, pre-26.x game versions with Fabric and Quilt require an
//! intermediary or hashed mappings library. However, 26.x and later don't
//! require these libraries, and don't have valid downloads for them. The same
//! loader version can be used on both sides of this boundary, but our v0
//! manifest files can't differentiate between their profiles.
//!
//! To fix this, v1 introduces the concept of *version groups*: game versions
//! before 26.x are version group v1, and 26.x and later are v2. Then, we
//! parameterize our version info on both version group and loader version,
//! letting us specify the right configuration based on both game version and
//! loader version.
//!
//! Why not parameterize on game version and loader version directly? Most game
//! versions have the same configuration as their surrounding game versions, so
//! we'd end up with many duplicate configurations: the number of game versions
//! multiplied by the number of loader versions.
//!
//! This file lets you configure what game versions are grouped together.
//!
//! Each version group is templated from a specific game version - e.g. game
//! version 1.21 is used as the template file for 1.20, 1.19, etc.

pub const UNIVERSAL_METADATA_GROUP: &str = "universal";
pub const LEGACY_METADATA_GROUP: &str = "v1";
pub const MODERN_METADATA_GROUP: &str = "v2";

pub struct MetadataGroup {
    pub id: &'static str,
    /// Minecraft version used to fetch and template this group's loader profiles.
    pub loader_profile_template_game_version: String,
    pub game_versions: Vec<String>,
}

pub fn metadata_groups<'a>(
    mod_loader: &str,
    game_versions: impl IntoIterator<Item = &'a str>,
) -> Vec<MetadataGroup> {
    if !uses_version_groups(mod_loader) {
        return vec![MetadataGroup {
            id: UNIVERSAL_METADATA_GROUP,
            loader_profile_template_game_version: "1.21".to_string(),
            game_versions: game_versions
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
        }];
    }

    let game_versions = game_versions.into_iter().collect::<Vec<_>>();
    let legacy_game_versions = game_versions
        .iter()
        .copied()
        .filter(|game_version| {
            metadata_group_id_for_game_version(mod_loader, game_version)
                == LEGACY_METADATA_GROUP
        })
        .collect::<Vec<_>>();
    let modern_game_versions = game_versions
        .iter()
        .copied()
        .filter(|game_version| {
            metadata_group_id_for_game_version(mod_loader, game_version)
                == MODERN_METADATA_GROUP
        })
        .collect::<Vec<_>>();

    let mut groups = Vec::new();

    if !legacy_game_versions.is_empty() {
        groups.push(MetadataGroup {
            id: LEGACY_METADATA_GROUP,
            loader_profile_template_game_version: legacy_game_versions
                .iter()
                .find(|x| **x == "1.21")
                .copied()
                .unwrap_or(legacy_game_versions[0])
                .to_string(),
            game_versions: legacy_game_versions
                .iter()
                .map(|x| x.to_string())
                .collect(),
        });
    }

    if !modern_game_versions.is_empty() {
        groups.push(MetadataGroup {
            id: MODERN_METADATA_GROUP,
            loader_profile_template_game_version: modern_game_versions[0]
                .to_string(),
            game_versions: modern_game_versions
                .iter()
                .map(|x| x.to_string())
                .collect(),
        });
    }

    groups
}

pub fn metadata_group_for_game_version<'a>(
    groups: &'a [MetadataGroup],
    mod_loader: &str,
    game_version: &str,
) -> Option<&'a MetadataGroup> {
    let group_id = metadata_group_id_for_game_version(mod_loader, game_version);

    groups.iter().find(|group| group.id == group_id)
}

fn metadata_group_id_for_game_version(
    mod_loader: &str,
    game_version: &str,
) -> &'static str {
    if uses_version_groups(mod_loader) && is_modern_game_version(game_version) {
        MODERN_METADATA_GROUP
    } else if uses_version_groups(mod_loader) {
        LEGACY_METADATA_GROUP
    } else {
        UNIVERSAL_METADATA_GROUP
    }
}

fn uses_version_groups(mod_loader: &str) -> bool {
    matches!(mod_loader, "fabric" | "quilt")
}

// Update these group boundaries if upstream loader profiles gain another
// structural incompatibility between Minecraft versions.
fn is_modern_game_version(game_version: &str) -> bool {
    let major = game_version
        .split(['.', 'w'])
        .next()
        .and_then(|x| x.parse::<usize>().ok());

    major.is_some_and(|x| x >= 26)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabric_and_quilt_split_profiles_at_26() {
        for mod_loader in ["fabric", "quilt"] {
            let groups = metadata_groups(
                mod_loader,
                ["26.2", "26w14a", "1.21.11", "1.21", "25w46a"],
            );

            assert_eq!(groups.len(), 2);
            assert_eq!(groups[0].id, LEGACY_METADATA_GROUP);
            assert_eq!(groups[0].loader_profile_template_game_version, "1.21");
            assert_eq!(groups[0].game_versions, ["1.21.11", "1.21", "25w46a"]);
            assert_eq!(groups[1].id, MODERN_METADATA_GROUP);
            assert_eq!(groups[1].loader_profile_template_game_version, "26.2");
            assert_eq!(groups[1].game_versions, ["26.2", "26w14a"]);
        }
    }

    #[test]
    fn other_loaders_keep_a_universal_profile() {
        let groups = metadata_groups("forge", ["26.2", "1.21"]);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, UNIVERSAL_METADATA_GROUP);
        assert_eq!(groups[0].loader_profile_template_game_version, "1.21");
        assert_eq!(groups[0].game_versions, ["26.2", "1.21"]);
    }
}
