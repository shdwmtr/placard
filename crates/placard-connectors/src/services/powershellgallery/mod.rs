use super::meta::{Param, PresetMeta};
mod downloads;
mod platform_support;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use platform_support::resolve_platform_support;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "powershellgallery-downloads",
        service: "powershellgallery",
        description: "",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "powershellgallery-platform-support",
        service: "powershellgallery",
        description: "",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_platform_support,
    },
    PresetMeta {
        preset: "powershellgallery-version",
        service: "powershellgallery",
        description: "",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
