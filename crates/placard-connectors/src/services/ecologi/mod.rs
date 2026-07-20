use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod carbon;
mod trees;

pub(crate) use carbon::resolve_carbon;
pub(crate) use trees::resolve_trees;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "ecologi-carbon",
        service: "ecologi",
        description: "Ecologi (Carbon Offset)",
        params: &[Param {
            name: "username",
            required: true,
            example: "ecologi",
        }],
        numeric: true,
        resolve: resolve_carbon,
    },
    PresetMeta {
        preset: "ecologi-trees",
        service: "ecologi",
        description: "Ecologi (Trees)",
        params: &[Param {
            name: "username",
            required: true,
            example: "ecologi",
        }],
        numeric: true,
        resolve: resolve_trees,
    },
];
