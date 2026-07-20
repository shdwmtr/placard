use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod license;
mod platform;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use license::resolve_license;
pub(crate) use platform::resolve_platform;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "conda-downloads",
        service: "conda",
        description: "Conda Downloads",
        params: &[
            Param {
                name: "channel",
                required: true,
                example: "conda-forge",
            },
            Param {
                name: "package",
                required: true,
                example: "python",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "conda-license",
        service: "conda",
        description: "Conda - License",
        params: &[
            Param {
                name: "channel",
                required: true,
                example: "conda-forge",
            },
            Param {
                name: "package",
                required: true,
                example: "mlforecast",
            },
        ],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "conda-platform",
        service: "conda",
        description: "Conda Platform",
        params: &[
            Param {
                name: "channel",
                required: true,
                example: "conda-forge",
            },
            Param {
                name: "package",
                required: true,
                example: "python",
            },
        ],
        numeric: false,
        resolve: resolve_platform,
    },
    PresetMeta {
        preset: "conda-version",
        service: "conda",
        description: "Conda Version",
        params: &[
            Param {
                name: "channel",
                required: true,
                example: "conda-forge",
            },
            Param {
                name: "package",
                required: true,
                example: "python",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
