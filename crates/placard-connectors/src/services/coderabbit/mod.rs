use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod pull_request;

pub(crate) use pull_request::resolve_pull_request;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "coderabbit-pull-request",
    service: "coderabbit",
    description: "CodeRabbit Pull Request Reviews",
    params: &[
        Param {
            name: "provider",
            required: true,
            example: "",
        },
        Param {
            name: "org",
            required: true,
            example: "coderabbitai",
        },
        Param {
            name: "repo",
            required: true,
            example: "ast-grep-essentials",
        },
    ],
    numeric: true,
    resolve: resolve_pull_request,
}];
