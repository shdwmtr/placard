use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod license;
mod version;

pub(crate) use license::resolve_license;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "cran-license",
        service: "cran",
        description: "",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "cran-version",
        service: "cran",
        description: "",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
