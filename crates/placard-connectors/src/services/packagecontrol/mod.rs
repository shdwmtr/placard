use super::meta::{Param, PresetMeta};
mod packagecontrol;

pub(crate) use packagecontrol::resolve_packagecontrol;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "packagecontrol",
    service: "packagecontrol",
    description: "Package Control Downloads",
    params: &[
        Param {
            name: "interval",
            required: true,
            example: "",
        },
        Param {
            name: "package",
            required: true,
            example: "",
        },
    ],
    numeric: true,
    resolve: resolve_packagecontrol,
}];
