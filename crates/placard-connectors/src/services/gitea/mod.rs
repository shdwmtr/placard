use super::meta::{Param, PresetMeta};
mod forks;
mod languages_count;
mod last_commit;
mod release;
mod stars;

pub(crate) use forks::resolve_forks;
pub(crate) use languages_count::resolve_languages_count;
pub(crate) use last_commit::resolve_last_commit;
pub(crate) use release::resolve_release;
pub(crate) use stars::resolve_stars;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "gitea-forks",
        service: "gitea",
        description: "Gitea Forks",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "gitea",
            },
            Param {
                name: "repo",
                required: true,
                example: "tea",
            },
            Param {
                name: "gitea-url",
                required: false,
                example: "https://gitea.com",
            },
        ],
        numeric: true,
        resolve: resolve_forks,
    },
    PresetMeta {
        preset: "gitea-languages-count",
        service: "gitea",
        description: "Gitea language count",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "gitea",
            },
            Param {
                name: "repo",
                required: true,
                example: "tea",
            },
            Param {
                name: "gitea-url",
                required: false,
                example: "https://gitea.com",
            },
        ],
        numeric: true,
        resolve: resolve_languages_count,
    },
    PresetMeta {
        preset: "gitea-last-commit",
        service: "gitea",
        description: "Gitea Last Commit",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "gitea",
            },
            Param {
                name: "repo",
                required: true,
                example: "tea",
            },
            Param {
                name: "gitea-url",
                required: false,
                example: "https://gitea.com",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
            Param {
                name: "path",
                required: false,
                example: "",
            },
            Param {
                name: "display-timestamp",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_last_commit,
    },
    PresetMeta {
        preset: "gitea-release",
        service: "gitea",
        description: "Gitea Release",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "gitea",
            },
            Param {
                name: "repo",
                required: true,
                example: "tea",
            },
            Param {
                name: "gitea-url",
                required: false,
                example: "https://gitea.com",
            },
            Param {
                name: "display-name",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_release,
    },
    PresetMeta {
        preset: "gitea-stars",
        service: "gitea",
        description: "Gitea Stars",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "gitea",
            },
            Param {
                name: "repo",
                required: true,
                example: "tea",
            },
            Param {
                name: "gitea-url",
                required: false,
                example: "https://gitea.com",
            },
        ],
        numeric: true,
        resolve: resolve_stars,
    },
];
