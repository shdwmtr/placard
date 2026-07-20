use super::meta::{Param, PresetMeta};
mod version;

pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "piwheels-version",
    service: "piwheels",
    description: "PiWheels Version",
    params: &[Param {
        name: "wheel",
        required: true,
        example: "flask",
    }],
    numeric: false,
    resolve: resolve_version,
}];
