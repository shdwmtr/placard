use super::meta::{Param, PresetMeta};
mod score;

pub(crate) use score::resolve_score;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "npms-io-score",
    service: "npms_io",
    description: "npms.io",
    params: &[
        Param {
            name: "package",
            required: true,
            example: "command",
        },
        Param {
            name: "type",
            required: true,
            example: "@vue",
        },
    ],
    numeric: false,
    resolve: resolve_score,
}];
