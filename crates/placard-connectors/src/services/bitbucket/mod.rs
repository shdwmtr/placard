use super::meta::{Param, PresetMeta};
mod issues;
mod last_commit;
mod pipelines;
mod pull_request;

pub(crate) use issues::resolve_issues;
pub(crate) use last_commit::resolve_last_commit;
pub(crate) use pipelines::resolve_pipelines;
pub(crate) use pull_request::resolve_pull_request;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "bitbucket-issues",
        service: "bitbucket",
        description: "",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "",
            },
            Param {
                name: "repo",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_issues,
    },
    PresetMeta {
        preset: "bitbucket-last-commit",
        service: "bitbucket",
        description: "Bitbucket last commit",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "shields-io",
            },
            Param {
                name: "repo",
                required: true,
                example: "test-repo",
            },
            Param {
                name: "branch",
                required: true,
                example: "main",
            },
            Param {
                name: "path",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_last_commit,
    },
    PresetMeta {
        preset: "bitbucket-pipelines",
        service: "bitbucket",
        description: "Bitbucket Pipelines",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "shields-io",
            },
            Param {
                name: "repo",
                required: true,
                example: "test-repo",
            },
            Param {
                name: "branch",
                required: true,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_pipelines,
    },
    PresetMeta {
        preset: "bitbucket-pull-request",
        service: "bitbucket",
        description: "",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "",
            },
            Param {
                name: "repo",
                required: true,
                example: "",
            },
            Param {
                name: "server",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_pull_request,
    },
];
