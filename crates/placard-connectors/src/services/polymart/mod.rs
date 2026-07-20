use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod latest_version;
mod rating;

pub(crate) use downloads::resolve_downloads;
pub(crate) use latest_version::resolve_latest_version;
pub(crate) use rating::resolve_rating;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "polymart-downloads",
        service: "polymart",
        description: "",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "polymart-latest-version",
        service: "polymart",
        description: "",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_latest_version,
    },
    PresetMeta {
        preset: "polymart-rating",
        service: "polymart",
        description: "",
        params: &[
            Param {
                name: "resource-id",
                required: true,
                example: "",
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
];
