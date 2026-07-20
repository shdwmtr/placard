use super::meta::{Param, PresetMeta};
mod license;
mod version;

pub(crate) use license::resolve_license;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "scoop-license",
        service: "scoop",
        description: "Scoop License",
        params: &[
            Param {
                name: "app",
                required: true,
                example: "ngrok",
            },
            Param {
                name: "bucket",
                required: false,
                example: "extras",
            },
        ],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "scoop-version",
        service: "scoop",
        description: "Scoop Version",
        params: &[
            Param {
                name: "app",
                required: true,
                example: "ngrok",
            },
            Param {
                name: "bucket",
                required: false,
                example: "extras",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
