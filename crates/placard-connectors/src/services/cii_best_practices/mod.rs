use super::meta::{Param, PresetMeta};
mod cii_best_practices;

pub(crate) use cii_best_practices::resolve_cii_best_practices;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "cii-best-practices",
    service: "cii_best_practices",
    description: "CII Best Practices",
    params: &[
        Param {
            name: "project-id",
            required: true,
            example: "1",
        },
        Param {
            name: "metric",
            required: false,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_cii_best_practices,
}];
