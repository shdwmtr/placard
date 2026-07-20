use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod rating;
mod users;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use rating::resolve_rating;
pub(crate) use users::resolve_users;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "amo-downloads",
        service: "amo",
        description: "Mozilla Add-on Downloads",
        params: &[Param {
            name: "addon_id",
            required: true,
            example: "dustman",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "amo-rating",
        service: "amo",
        description: "Mozilla Add-on Rating",
        params: &[Param {
            name: "addon_id",
            required: true,
            example: "dustman",
        }],
        numeric: true,
        resolve: resolve_rating,
    },
    PresetMeta {
        preset: "amo-users",
        service: "amo",
        description: "Mozilla Add-on Users",
        params: &[Param {
            name: "addon_id",
            required: true,
            example: "dustman",
        }],
        numeric: true,
        resolve: resolve_users,
    },
    PresetMeta {
        preset: "amo-version",
        service: "amo",
        description: "Mozilla Add-on Version",
        params: &[Param {
            name: "addon_id",
            required: true,
            example: "dustman",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
