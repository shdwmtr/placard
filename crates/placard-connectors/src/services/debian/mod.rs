use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod debian;

pub(crate) use debian::resolve_debian;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "debian",
    service: "debian",
    description: "Debian package (for distribution)",
    params: &[
        Param {
            name: "package",
            required: true,
            example: "apt",
        },
        Param {
            name: "distribution",
            required: true,
            example: "unstable",
        },
    ],
    numeric: false,
    resolve: resolve_debian,
}];
