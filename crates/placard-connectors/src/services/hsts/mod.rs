use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod hsts;

pub(crate) use hsts::resolve_hsts;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "hsts",
    service: "hsts",
    description: "Chromium HSTS preload",
    params: &[Param {
        name: "domain",
        required: true,
        example: "github.com",
    }],
    numeric: false,
    resolve: resolve_hsts,
}];
