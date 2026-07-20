use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod rating;
mod release_date;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use rating::resolve_rating;
pub(crate) use release_date::resolve_release_date;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "open-vsx-downloads",
        service: "open_vsx",
        description: "Open VSX Downloads",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "redhat",
            },
            Param {
                name: "extension",
                required: true,
                example: "java",
            },
            Param {
                name: "version",
                required: false,
                example: "0.69.0",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "open-vsx-rating",
        service: "open_vsx",
        description: "Open VSX Rating",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "redhat",
            },
            Param {
                name: "extension",
                required: true,
                example: "java",
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
        preset: "open-vsx-release-date",
        service: "open_vsx",
        description: "Open VSX Release Date",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "redhat",
            },
            Param {
                name: "extension",
                required: true,
                example: "java",
            },
        ],
        numeric: false,
        resolve: resolve_release_date,
    },
    PresetMeta {
        preset: "open-vsx-version",
        service: "open_vsx",
        description: "Open VSX Version",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "redhat",
            },
            Param {
                name: "extension",
                required: true,
                example: "java",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
