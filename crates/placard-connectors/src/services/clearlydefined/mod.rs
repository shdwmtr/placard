use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod score;

pub(crate) use score::resolve_score;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "clearlydefined-score",
    service: "clearlydefined",
    description: "ClearlyDefined Score",
    params: &[
        Param {
            name: "type",
            required: true,
            example: "npm",
        },
        Param {
            name: "provider",
            required: true,
            example: "npmjs",
        },
        Param {
            name: "namespace",
            required: true,
            example: "-",
        },
        Param {
            name: "name",
            required: true,
            example: "jquery",
        },
        Param {
            name: "revision",
            required: true,
            example: "3.4.1",
        },
    ],
    numeric: true,
    resolve: resolve_score,
}];
