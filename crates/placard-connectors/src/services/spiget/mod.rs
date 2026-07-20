use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod download_size;
mod downloads;
mod latest_version;
mod rating;
mod tested_versions;

pub(crate) use download_size::resolve_download_size;
pub(crate) use downloads::resolve_downloads;
pub(crate) use latest_version::resolve_latest_version;
pub(crate) use rating::resolve_rating;
pub(crate) use tested_versions::resolve_tested_versions;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "spiget-download-size",
        service: "spiget",
        description: "Spiget Download Size",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "15904",
        }],
        numeric: false,
        resolve: resolve_download_size,
    },
    PresetMeta {
        preset: "spiget-downloads",
        service: "spiget",
        description: "Spiget Downloads",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "9089",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "spiget-latest-version",
        service: "spiget",
        description: "Spiget Version",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "9089",
        }],
        numeric: false,
        resolve: resolve_latest_version,
    },
    PresetMeta {
        preset: "spiget-rating",
        service: "spiget",
        description: "Spiget Rating",
        params: &[
            Param {
                name: "resource-id",
                required: true,
                example: "9089",
            },
            Param {
                name: "format",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_rating,
    },
    PresetMeta {
        preset: "spiget-tested-versions",
        service: "spiget",
        description: "Spiget tested server versions",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "9089",
        }],
        numeric: false,
        resolve: resolve_tested_versions,
    },
];
