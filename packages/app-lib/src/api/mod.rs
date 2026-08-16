//! API for interacting with Theseus
pub mod ai;
pub mod cache;
pub mod content_search;
pub mod curseforge;
pub mod drop_classifier;
pub mod friends;
pub mod google_ip;
pub mod handler;
pub mod hongshi;
pub mod instance;
pub mod jre;
pub(crate) mod loader_metadata;
pub mod logs;
pub mod memory;
pub mod metadata;
pub mod minecraft_auth;
pub mod minecraft_news;
pub mod minecraft_skins;
pub mod mr_auth;
pub mod multiplayer;
pub mod pack;
pub mod process;
pub mod server_address;
pub mod settings;
pub mod symlink;
pub mod tags;
pub mod terracotta;
pub mod translation;
pub mod worlds;

pub mod data {
    pub use crate::state::{
        AppliedContentSetPatch, CacheBehaviour, CacheValueType, CachedEntry,
        ContentFile, ContentItem, ContentItemCapabilities, ContentItemOwner,
        ContentItemProject, ContentItemVersion, ContentOwnershipKind,
        ContentProvider, ContentProviderRef, ContentUpdatePlan,
        ContentUpdatePlanAction, ContentUpdateResolution,
        ContentUpdateResolutionChoice, ContentUpdateScope, CreateInstance,
        Credentials, Dependency, DirectoryInfo, EditInstance, Hooks,
        InstanceContentPack, InstanceContentSnapshot,
        InstanceContentSnapshotItem, InstanceContentWarning,
        InstanceInstallCandidate, InstanceInstallTarget,
        InstanceLaunchOverridesPatch, InstanceLink, InstanceMetadata,
        JavaVersion, LinkedModpackInfo, ManualDownloadOperationKind,
        ManualDownloadState, MemorySettings, ModLoader, ModrinthCredentials,
        Organization, OwnerType, PackMemberMaterializationState,
        PackMemberOverrideKind, PendingManualDownload, PrivacySettings,
        ProcessMetadata, Project, ProjectType, ProjectV3, SearchResult,
        SearchResults, SearchResultsV3, Settings, TeamMember, Theme, User,
        UserFriend, Version, WindowSize,
    };
    pub use ariadne::users::UserStatus;
    pub use modrinth_content_management::{
        ContentType, ResolutionPreferences, ResolveContentPlan,
        ResolveContentRequest,
    };
}

pub mod prelude {
    pub use crate::{
        State, ai,
        data::*,
        event::CommandPayload,
        install, instance,
        jre::{self, JdkVersionInfo},
        metadata, minecraft_auth, mr_auth, pack, process, settings,
        state::{ReleaseChannel, db_backup::app_db_backup_dir},
        translation,
        util::{
            io::{IOError, canonicalize},
            network::{is_network_metered, tcp_listen_any_loopback},
        },
    };
}
