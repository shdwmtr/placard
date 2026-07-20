use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod version;

pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "hackage-version",
    service: "hackage",
    description: "Hackage Version",
    params: &[Param {
        name: "package-name",
        required: true,
        example: "lens",
    }],
    numeric: false,
    resolve: resolve_version,
}];
