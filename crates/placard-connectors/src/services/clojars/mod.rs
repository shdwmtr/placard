use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "clojars-downloads",
        service: "clojars",
        description: "Clojars Downloads",
        params: &[Param {
            name: "clojar",
            required: true,
            example: "prismic",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "clojars-version",
        service: "clojars",
        description: "Clojars Version",
        params: &[Param {
            name: "clojar",
            required: true,
            example: "prismic",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
