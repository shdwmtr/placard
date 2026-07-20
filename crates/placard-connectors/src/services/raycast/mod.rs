use super::meta::{Param, PresetMeta};
mod installs;

pub(crate) use installs::resolve_installs;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "installs",
    service: "raycast",
    description: "Raycast extension downloads count",
    params: &[
        Param {
            name: "user",
            required: true,
            example: "Fatpandac",
        },
        Param {
            name: "extension",
            required: true,
            example: "bilibili",
        },
    ],
    numeric: true,
    resolve: resolve_installs,
}];
