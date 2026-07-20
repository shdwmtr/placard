use super::meta::{Param, PresetMeta};
mod downloads;
mod favorites;
mod license;
mod update;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use favorites::resolve_favorites;
pub(crate) use license::resolve_license;
pub(crate) use update::resolve_update;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "eclipse-marketplace-downloads",
        service: "eclipse_marketplace",
        description: "Eclipse Marketplace Downloads",
        params: &[
            Param {
                name: "interval",
                required: true,
                example: "",
            },
            Param {
                name: "name",
                required: true,
                example: "planet-themes",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "eclipse-marketplace-favorites",
        service: "eclipse_marketplace",
        description: "Eclipse Marketplace Favorites",
        params: &[Param {
            name: "name",
            required: true,
            example: "notepad4e",
        }],
        numeric: true,
        resolve: resolve_favorites,
    },
    PresetMeta {
        preset: "eclipse-marketplace-license",
        service: "eclipse_marketplace",
        description: "Eclipse Marketplace License",
        params: &[Param {
            name: "name",
            required: true,
            example: "notepad4e",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "eclipse-marketplace-update",
        service: "eclipse_marketplace",
        description: "Eclipse Marketplace Last Update",
        params: &[Param {
            name: "name",
            required: true,
            example: "notepad4e",
        }],
        numeric: false,
        resolve: resolve_update,
    },
    PresetMeta {
        preset: "eclipse-marketplace-version",
        service: "eclipse_marketplace",
        description: "Eclipse Marketplace Version",
        params: &[Param {
            name: "name",
            required: true,
            example: "notepad4e",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
