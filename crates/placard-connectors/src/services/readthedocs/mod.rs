use super::meta::{Param, PresetMeta};
mod readthedocs;

pub(crate) use readthedocs::resolve_readthedocs;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "readthedocs",
    service: "readthedocs",
    description: "Read the Docs",
    params: &[
        Param {
            name: "project",
            required: true,
            example: "pip",
        },
        Param {
            name: "version",
            required: false,
            example: "stable",
        },
    ],
    numeric: false,
    resolve: resolve_readthedocs,
}];
