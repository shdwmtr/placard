use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod followers;
mod posts;

pub(crate) use followers::resolve_followers;
pub(crate) use posts::resolve_posts;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "bluesky-followers",
        service: "bluesky",
        description: "Bluesky followers",
        params: &[Param {
            name: "actor",
            required: true,
            example: "chitvs.bsky.social",
        }],
        numeric: true,
        resolve: resolve_followers,
    },
    PresetMeta {
        preset: "bluesky-posts",
        service: "bluesky",
        description: "Bluesky posts",
        params: &[Param {
            name: "actor",
            required: true,
            example: "chitvs.bsky.social",
        }],
        numeric: true,
        resolve: resolve_posts,
    },
];
