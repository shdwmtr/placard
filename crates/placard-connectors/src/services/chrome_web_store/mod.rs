use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod last_updated;
mod rating;
mod rating_count;
mod size;
mod users;
mod version;

pub(crate) use last_updated::resolve_last_updated;
pub(crate) use rating::resolve_rating;
pub(crate) use rating_count::resolve_rating_count;
pub(crate) use size::resolve_size;
pub(crate) use users::resolve_users;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "chrome-web-store-last-updated",
        service: "chrome_web_store",
        description: "Chrome Web Store Last Updated",
        params: &[Param {
            name: "store-id",
            required: true,
            example: "nccfelhkfpbnefflolffkclhenplhiab",
        }],
        numeric: false,
        resolve: resolve_last_updated,
    },
    PresetMeta {
        preset: "chrome-web-store-rating",
        service: "chrome_web_store",
        description: "Chrome Web Store Rating",
        params: &[Param {
            name: "store-id",
            required: true,
            example: "ogffaloegjglncjfehdfplabnoondfjo",
        }],
        numeric: true,
        resolve: resolve_rating,
    },
    PresetMeta {
        preset: "chrome-web-store-rating-count",
        service: "chrome_web_store",
        description: "Chrome Web Store Rating Count",
        params: &[Param {
            name: "store-id",
            required: true,
            example: "ogffaloegjglncjfehdfplabnoondfjo",
        }],
        numeric: true,
        resolve: resolve_rating_count,
    },
    PresetMeta {
        preset: "chrome-web-store-size",
        service: "chrome_web_store",
        description: "Chrome Web Store Size",
        params: &[Param {
            name: "store-id",
            required: true,
            example: "nccfelhkfpbnefflolffkclhenplhiab",
        }],
        numeric: false,
        resolve: resolve_size,
    },
    PresetMeta {
        preset: "chrome-web-store-users",
        service: "chrome_web_store",
        description: "Chrome Web Store Users",
        params: &[Param {
            name: "store-id",
            required: true,
            example: "ogffaloegjglncjfehdfplabnoondfjo",
        }],
        numeric: true,
        resolve: resolve_users,
    },
    PresetMeta {
        preset: "chrome-web-store-version",
        service: "chrome_web_store",
        description: "Chrome Web Store Version",
        params: &[Param {
            name: "store-id",
            required: true,
            example: "ogffaloegjglncjfehdfplabnoondfjo",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
