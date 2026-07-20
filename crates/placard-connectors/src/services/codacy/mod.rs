use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod coverage;
mod grade;

pub(crate) use coverage::resolve_coverage;
pub(crate) use grade::resolve_grade;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "codacy-coverage",
        service: "codacy",
        description: "Codacy coverage",
        params: &[
            Param {
                name: "project-id",
                required: true,
                example: "84c0a068ce9349f2bcaa07b5977bd932",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_coverage,
    },
    PresetMeta {
        preset: "codacy-grade",
        service: "codacy",
        description: "Codacy grade",
        params: &[
            Param {
                name: "project-id",
                required: true,
                example: "0cb32ce695b743d68257021455330c66",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_grade,
    },
];
