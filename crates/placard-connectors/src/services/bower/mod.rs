use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod license;
mod version;

pub(crate) use license::resolve_license;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "bower-license",
        service: "bower",
        description: "Bower License",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "bootstrap",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "bower-version",
        service: "bower",
        description: "Bower Version",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "bootstrap",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
