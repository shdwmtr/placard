use super::meta::{Param, PresetMeta};
mod validation;

pub(crate) use validation::resolve_validation;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "w3c-validation",
    service: "w3c",
    description: "W3C Validation",
    params: &[
        Param {
            name: "target-url",
            required: true,
            example: "https://validator.nu/",
        },
        Param {
            name: "parser",
            required: true,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_validation,
}];
