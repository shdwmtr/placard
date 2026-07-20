use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod license;
mod version;

pub(crate) use license::resolve_license;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "ctan-license",
        service: "ctan",
        description: "",
        params: &[Param {
            name: "library",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "ctan-version",
        service: "ctan",
        description: "",
        params: &[Param {
            name: "library",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
