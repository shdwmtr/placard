use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod version;

pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "jsr-version",
    service: "jsr",
    description: "JSR Version",
    params: &[
        Param {
            name: "scope",
            required: true,
            example: "@luca",
        },
        Param {
            name: "package-name",
            required: true,
            example: "flag",
        },
    ],
    numeric: false,
    resolve: resolve_version,
}];
