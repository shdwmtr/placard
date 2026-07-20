use super::meta::{Param, PresetMeta};
mod version;

pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "sdkman-version",
    service: "sdkman",
    description: "SDKMAN Version",
    params: &[Param {
        name: "candidate",
        required: true,
        example: "java",
    }],
    numeric: false,
    resolve: resolve_version,
}];
