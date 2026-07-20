use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod depfu;

pub(crate) use depfu::resolve_depfu;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "depfu",
    service: "depfu",
    description: "Depfu",
    params: &[
        Param {
            name: "vcs-type",
            required: true,
            example: "",
        },
        Param {
            name: "user",
            required: true,
            example: "",
        },
        Param {
            name: "repo",
            required: true,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_depfu,
}];
