use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod likes;
mod points;
mod publisher;

pub(crate) use downloads::resolve_downloads;
pub(crate) use likes::resolve_likes;
pub(crate) use points::resolve_points;
pub(crate) use publisher::resolve_publisher;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "pub-downloads",
        service: "pub",
        description: "Pub Monthly Downloads",
        params: &[Param {
            name: "package",
            required: true,
            example: "analysis_options",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "pub-likes",
        service: "pub",
        description: "Pub Likes",
        params: &[Param {
            name: "package",
            required: true,
            example: "analysis_options",
        }],
        numeric: true,
        resolve: resolve_likes,
    },
    PresetMeta {
        preset: "pub-points",
        service: "pub",
        description: "Pub Points",
        params: &[Param {
            name: "package",
            required: true,
            example: "analysis_options",
        }],
        numeric: false,
        resolve: resolve_points,
    },
    PresetMeta {
        preset: "pub-publisher",
        service: "pub",
        description: "Pub Publisher",
        params: &[Param {
            name: "package",
            required: true,
            example: "path",
        }],
        numeric: false,
        resolve: resolve_publisher,
    },
];
