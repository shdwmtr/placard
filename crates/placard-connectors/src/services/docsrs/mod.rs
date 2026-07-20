use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod docsrs;

pub(crate) use docsrs::resolve_docsrs;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "docsrs",
    service: "docsrs",
    description: "docs.rs (with version)",
    params: &[
        Param {
            name: "crate",
            required: true,
            example: "regex",
        },
        Param {
            name: "version",
            required: true,
            example: "latest",
        },
    ],
    numeric: false,
    resolve: resolve_docsrs,
}];
