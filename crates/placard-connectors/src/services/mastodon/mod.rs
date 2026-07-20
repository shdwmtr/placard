use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod follow;

pub(crate) use follow::resolve_follow;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "mastodon-follow",
    service: "mastodon",
    description: "Mastodon Follow",
    params: &[
        Param {
            name: "id",
            required: true,
            example: "26471",
        },
        Param {
            name: "domain",
            required: false,
            example: "mastodon.social",
        },
    ],
    numeric: true,
    resolve: resolve_follow,
}];
