use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "nuget-downloads",
        service: "nuget",
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
        preset: "nuget-version",
        service: "nuget",
        description: "",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "",
            },
            Param {
                name: "variant",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
