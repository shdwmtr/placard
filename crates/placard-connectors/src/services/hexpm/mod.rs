use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod license;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use license::resolve_license;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "hexpm-downloads",
        service: "hexpm",
        description: "",
        params: &[
            Param {
                name: "interval",
                required: true,
                example: "",
            },
            Param {
                name: "package-name",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "hexpm-license",
        service: "hexpm",
        description: "",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "hexpm-version",
        service: "hexpm",
        description: "",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
