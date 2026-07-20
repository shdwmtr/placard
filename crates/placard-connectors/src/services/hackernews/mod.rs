use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod user_karma;

pub(crate) use user_karma::resolve_user_karma;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "hackernews-user-karma",
    service: "hackernews",
    description: "HackerNews User Karma",
    params: &[Param {
        name: "id",
        required: true,
        example: "pg",
    }],
    numeric: true,
    resolve: resolve_user_karma,
}];
