use super::meta::{Param, PresetMeta};
mod test_count;
mod test_pass_ratio;
mod test_summary;

pub(crate) use test_count::resolve_test_count;
pub(crate) use test_pass_ratio::resolve_test_pass_ratio;
pub(crate) use test_summary::resolve_test_summary;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "testspace-test-count",
        service: "testspace",
        description: "Testspace tests count",
        params: &[
            Param {
                name: "metric",
                required: true,
                example: "",
            },
            Param {
                name: "org",
                required: true,
                example: "swellaby",
            },
            Param {
                name: "project",
                required: true,
                example: "swellaby:testspace-sample",
            },
            Param {
                name: "space",
                required: true,
                example: "main",
            },
        ],
        numeric: true,
        resolve: resolve_test_count,
    },
    PresetMeta {
        preset: "testspace-test-pass-ratio",
        service: "testspace",
        description: "Testspace pass ratio",
        params: &[
            Param {
                name: "org",
                required: true,
                example: "swellaby",
            },
            Param {
                name: "project",
                required: true,
                example: "swellaby:testspace-sample",
            },
            Param {
                name: "space",
                required: true,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_test_pass_ratio,
    },
    PresetMeta {
        preset: "testspace-test-summary",
        service: "testspace",
        description: "Testspace tests",
        params: &[
            Param {
                name: "org",
                required: true,
                example: "swellaby",
            },
            Param {
                name: "project",
                required: true,
                example: "swellaby:testspace-sample",
            },
            Param {
                name: "space",
                required: true,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_test_summary,
    },
];
