use super::meta::{Param, PresetMeta};
mod cdnjs;

pub(crate) use cdnjs::resolve_cdnjs;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "cdnjs",
    service: "cdnjs",
    description: "Cdnjs",
    params: &[Param {
        name: "library",
        required: true,
        example: "jquery",
    }],
    numeric: false,
    resolve: resolve_cdnjs,
}];
