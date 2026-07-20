use super::meta::{Param, PresetMeta};
mod revolt;

pub(crate) use revolt::resolve_revolt;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "revolt",
    service: "revolt",
    description: "Revolt",
    params: &[
        Param {
            name: "invite-id",
            required: true,
            example: "01F7ZSBSFHQ8TA81725KQCSDDP",
        },
        Param {
            name: "revolt-api-url",
            required: false,
            example: "https://api.revolt.chat",
        },
    ],
    numeric: true,
    resolve: resolve_revolt,
}];
