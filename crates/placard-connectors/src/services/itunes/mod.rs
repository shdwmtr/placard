use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod itunes;

pub(crate) use itunes::resolve_itunes;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "itunes",
    service: "itunes",
    description: "iTunes App Store",
    params: &[Param {
        name: "bundle-id",
        required: true,
        example: "803453959",
    }],
    numeric: false,
    resolve: resolve_itunes,
}];
