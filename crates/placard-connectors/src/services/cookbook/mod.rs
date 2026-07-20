use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod cookbook;

pub(crate) use cookbook::resolve_cookbook;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "cookbook",
    service: "cookbook",
    description: "Chef cookbook",
    params: &[Param {
        name: "cookbook",
        required: true,
        example: "chef-sugar",
    }],
    numeric: false,
    resolve: resolve_cookbook,
}];
