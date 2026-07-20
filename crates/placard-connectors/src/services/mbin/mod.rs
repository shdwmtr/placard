use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod mbin;

pub(crate) use mbin::resolve_mbin;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "mbin",
    service: "mbin",
    description: "Mbin",
    params: &[Param {
        name: "magazine",
        required: true,
        example: "kbinEarth@kbin.earth",
    }],
    numeric: true,
    resolve: resolve_mbin,
}];
