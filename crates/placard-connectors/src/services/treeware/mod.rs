use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod trees;

pub(crate) use trees::resolve_trees;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "treeware-trees",
    service: "treeware",
    description: "Treeware (Trees)",
    params: &[
        Param {
            name: "owner",
            required: true,
            example: "stoplightio",
        },
        Param {
            name: "package-name",
            required: true,
            example: "spectral",
        },
    ],
    numeric: true,
    resolve: resolve_trees,
}];
