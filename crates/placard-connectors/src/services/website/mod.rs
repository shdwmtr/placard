use super::meta::{Param, PresetMeta};
mod website;

pub(crate) use website::resolve_website;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "website",
    service: "website",
    description: "Website",
    params: &[
        Param {
            name: "url",
            required: true,
            example: "https://shields.io",
        },
        Param {
            name: "up_message",
            required: false,
            example: "",
        },
        Param {
            name: "down_message",
            required: false,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_website,
}];
