use super::meta::{Param, PresetMeta};
mod ossf_scorecard;

pub(crate) use ossf_scorecard::resolve_ossf_scorecard;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "ossf-scorecard",
    service: "ossf_scorecard",
    description: "OSSF-Scorecard Score",
    params: &[
        Param {
            name: "host",
            required: true,
            example: "github.com",
        },
        Param {
            name: "org-name",
            required: true,
            example: "rohankh532",
        },
        Param {
            name: "repo-name",
            required: true,
            example: "org-workflow-add",
        },
    ],
    numeric: true,
    resolve: resolve_ossf_scorecard,
}];
