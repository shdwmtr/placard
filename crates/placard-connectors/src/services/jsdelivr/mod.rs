use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod hits_github;
mod hits_npm;

pub(crate) use hits_github::resolve_hits_github;
pub(crate) use hits_npm::resolve_hits_npm;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "jsdelivr-hits-github",
        service: "jsdelivr",
        description: "jsDelivr hits (GitHub)",
        params: &[
            Param {
                name: "period",
                required: true,
                example: "",
            },
            Param {
                name: "owner",
                required: true,
                example: "jquery",
            },
            Param {
                name: "repo",
                required: true,
                example: "jquery",
            },
        ],
        numeric: true,
        resolve: resolve_hits_github,
    },
    PresetMeta {
        preset: "jsdelivr-hits-npm",
        service: "jsdelivr",
        description: "jsDelivr hits (npm)",
        params: &[
            Param {
                name: "period",
                required: true,
                example: "",
            },
            Param {
                name: "package",
                required: true,
                example: "",
            },
            Param {
                name: "scope",
                required: true,
                example: "@angular",
            },
        ],
        numeric: true,
        resolve: resolve_hits_npm,
    },
];
