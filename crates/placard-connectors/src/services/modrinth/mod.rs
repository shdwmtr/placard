use super::meta::{Param, PresetMeta};
mod downloads;
mod followers;
mod game_versions;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use followers::resolve_followers;
pub(crate) use game_versions::resolve_game_versions;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "modrinth-downloads",
        service: "modrinth",
        description: "Modrinth Downloads",
        params: &[Param {
            name: "project-id",
            required: true,
            example: "AANobbMI",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "modrinth-followers",
        service: "modrinth",
        description: "Modrinth Followers",
        params: &[Param {
            name: "project-id",
            required: true,
            example: "AANobbMI",
        }],
        numeric: true,
        resolve: resolve_followers,
    },
    PresetMeta {
        preset: "modrinth-game-versions",
        service: "modrinth",
        description: "Modrinth Game Versions",
        params: &[Param {
            name: "project-id",
            required: true,
            example: "AANobbMI",
        }],
        numeric: false,
        resolve: resolve_game_versions,
    },
    PresetMeta {
        preset: "modrinth-version",
        service: "modrinth",
        description: "Modrinth Version",
        params: &[Param {
            name: "project-id",
            required: true,
            example: "AANobbMI",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
