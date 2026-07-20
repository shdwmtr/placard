use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;

pub(crate) use downloads::resolve_downloads;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "npm-stat-downloads",
    service: "npm_stat",
    description: "NPM Downloads by package author",
    params: &[
        Param {
            name: "author",
            required: true,
            example: "dukeluo",
        },
        Param {
            name: "interval",
            required: true,
            example: "",
        },
    ],
    numeric: true,
    resolve: resolve_downloads,
}];
