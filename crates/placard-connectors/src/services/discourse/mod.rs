use super::meta::{Param, PresetMeta};
mod discourse;

pub(crate) use discourse::resolve_discourse;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "discourse",
    service: "discourse",
    description: "Discourse Topics",
    params: &[
        Param {
            name: "server",
            required: true,
            example: "https://meta.discourse.org",
        },
        Param {
            name: "variant",
            required: true,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_discourse,
}];
