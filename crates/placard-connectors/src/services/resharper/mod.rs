use super::meta::{Param, PresetMeta};
mod resharper;

pub(crate) use resharper::resolve_resharper;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "resharper",
    service: "resharper",
    description: "",
    params: &[Param {
        name: "package-name",
        required: true,
        example: "",
    }],
    numeric: false,
    resolve: resolve_resharper,
}];
