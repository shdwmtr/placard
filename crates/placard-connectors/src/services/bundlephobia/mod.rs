use super::meta::{Param, PresetMeta};
mod bundlephobia;

pub(crate) use bundlephobia::resolve_bundlephobia;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "bundlephobia",
    service: "bundlephobia",
    description: "npm bundle size",
    params: &[
        Param {
            name: "package",
            required: true,
            example: "",
        },
        Param {
            name: "scope",
            required: false,
            example: "@cycle",
        },
        Param {
            name: "version",
            required: false,
            example: "15.0.0",
        },
        Param {
            name: "format",
            required: false,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_bundlephobia,
}];
