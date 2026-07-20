use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod rating;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use rating::resolve_rating;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "jetbrains-downloads",
        service: "jetbrains",
        description: "JetBrains Plugin Downloads",
        params: &[Param {
            name: "plugin-id",
            required: true,
            example: "1347",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "jetbrains-rating",
        service: "jetbrains",
        description: "JetBrains Plugin Rating",
        params: &[
            Param {
                name: "plugin-id",
                required: true,
                example: "11941",
            },
            Param {
                name: "format",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_rating,
    },
    PresetMeta {
        preset: "jetbrains-version",
        service: "jetbrains",
        description: "JetBrains Plugin Version",
        params: &[Param {
            name: "plugin-id",
            required: true,
            example: "9630",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
