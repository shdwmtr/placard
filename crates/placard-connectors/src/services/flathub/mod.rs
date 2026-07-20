use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "flathub-downloads",
        service: "flathub",
        description: "Flathub Downloads",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "org.mozilla.firefox",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "flathub-version",
        service: "flathub",
        description: "Flathub Version",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "org.mozilla.firefox",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
