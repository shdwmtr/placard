use super::meta::{Param, PresetMeta};
mod downloads;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "gnome-extensions-downloads",
        service: "gnome_extensions",
        description: "Gnome Extensions Downloads",
        params: &[Param {
            name: "extension-id",
            required: true,
            example: "just-perfection-desktop@just-perfection",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "gnome-extensions-version",
        service: "gnome_extensions",
        description: "Gnome Extensions Version",
        params: &[Param {
            name: "extension-id",
            required: true,
            example: "just-perfection-desktop@just-perfection",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
