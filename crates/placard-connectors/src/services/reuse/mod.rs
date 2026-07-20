use super::meta::{Param, PresetMeta};
mod compliance;

pub(crate) use compliance::resolve_compliance;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "reuse-compliance",
    service: "reuse",
    description: "REUSE Compliance",
    params: &[Param {
        name: "remote",
        required: true,
        example: "github.com/fsfe/reuse-tool",
    }],
    numeric: false,
    resolve: resolve_compliance,
}];
