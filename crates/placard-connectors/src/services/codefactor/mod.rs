use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod grade;

pub(crate) use grade::resolve_grade;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "codefactor-grade",
    service: "codefactor",
    description: "CodeFactor Grade (with branch)",
    params: &[
        Param {
            name: "vcs-type",
            required: true,
            example: "",
        },
        Param {
            name: "user",
            required: true,
            example: "microsoft",
        },
        Param {
            name: "repo",
            required: true,
            example: "powertoys",
        },
        Param {
            name: "branch",
            required: false,
            example: "main",
        },
    ],
    numeric: false,
    resolve: resolve_grade,
}];
