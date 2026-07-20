use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod last_update;
mod platform;
mod rating;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use last_update::resolve_last_update;
pub(crate) use platform::resolve_platform;
pub(crate) use rating::resolve_rating;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "wordpress-downloads",
        service: "wordpress",
        description: "",
        params: &[
            Param {
                name: "type",
                required: true,
                example: "",
            },
            Param {
                name: "slug",
                required: true,
                example: "",
            },
            Param {
                name: "interval",
                required: false,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "wordpress-last-update",
        service: "wordpress",
        description: "",
        params: &[
            Param {
                name: "type",
                required: true,
                example: "",
            },
            Param {
                name: "slug",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_last_update,
    },
    PresetMeta {
        preset: "wordpress-platform",
        service: "wordpress",
        description: "",
        params: &[
            Param {
                name: "type",
                required: true,
                example: "",
            },
            Param {
                name: "slug",
                required: true,
                example: "",
            },
            Param {
                name: "variant",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_platform,
    },
    PresetMeta {
        preset: "wordpress-rating",
        service: "wordpress",
        description: "",
        params: &[
            Param {
                name: "type",
                required: true,
                example: "",
            },
            Param {
                name: "slug",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_rating,
    },
    PresetMeta {
        preset: "wordpress-version",
        service: "wordpress",
        description: "",
        params: &[
            Param {
                name: "type",
                required: true,
                example: "",
            },
            Param {
                name: "slug",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
