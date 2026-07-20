use super::meta::{Param, PresetMeta};
mod downloads;
mod factorio_version;
mod last_updated;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use factorio_version::resolve_factorio_version;
pub(crate) use last_updated::resolve_last_updated;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "factorio-mod-portal-downloads",
        service: "factorio_mod_portal",
        description: "",
        params: &[Param {
            name: "mod-name",
            required: true,
            example: "",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "factorio-mod-portal-factorio-version",
        service: "factorio_mod_portal",
        description: "Factorio Mod Portal factorio versions",
        params: &[Param {
            name: "mod-name",
            required: true,
            example: "rso-mod",
        }],
        numeric: false,
        resolve: resolve_factorio_version,
    },
    PresetMeta {
        preset: "factorio-mod-portal-last-updated",
        service: "factorio_mod_portal",
        description: "Factorio Mod Portal last updated",
        params: &[Param {
            name: "mod-name",
            required: true,
            example: "rso-mod",
        }],
        numeric: false,
        resolve: resolve_last_updated,
    },
    PresetMeta {
        preset: "factorio-mod-portal-version",
        service: "factorio_mod_portal",
        description: "",
        params: &[Param {
            name: "mod-name",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
