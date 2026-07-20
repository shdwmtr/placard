use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod netlify;

pub(crate) use netlify::resolve_netlify;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "netlify",
    service: "netlify",
    description: "Netlify",
    params: &[Param {
        name: "project-id",
        required: true,
        example: "e6d5a4e0-dee1-4261-833e-2f47f509c68f",
    }],
    numeric: false,
    resolve: resolve_netlify,
}];
