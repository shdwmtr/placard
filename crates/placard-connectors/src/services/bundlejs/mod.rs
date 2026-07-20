use super::meta::{Param, PresetMeta};
mod package;

pub(crate) use package::resolve_package;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "bundlejs-package",
    service: "bundlejs",
    description: "npm package minimized gzipped size",
    params: &[
        Param {
            name: "package",
            required: true,
            example: "",
        },
        Param {
            name: "scope",
            required: false,
            example: "@ngneat",
        },
    ],
    numeric: false,
    resolve: resolve_package,
}];
