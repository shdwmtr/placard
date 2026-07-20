use super::meta::{Param, PresetMeta};
mod forks;
mod issues;
mod languages_count;
mod last_commit;
mod license;
mod pipeline_coverage;
mod pipeline_status;
mod release;
mod stars;
mod tag;
mod top_language;

pub(crate) use forks::resolve_forks;
pub(crate) use issues::resolve_issues;
pub(crate) use languages_count::resolve_languages_count;
pub(crate) use last_commit::resolve_last_commit;
pub(crate) use license::resolve_license;
pub(crate) use pipeline_coverage::resolve_pipeline_coverage;
pub(crate) use pipeline_status::resolve_pipeline_status;
pub(crate) use release::resolve_release;
pub(crate) use stars::resolve_stars;
pub(crate) use tag::resolve_tag;
pub(crate) use top_language::resolve_top_language;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "gitlab-forks",
        service: "gitlab",
        description: "GitLab Forks",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
        ],
        numeric: true,
        resolve: resolve_forks,
    },
    PresetMeta {
        preset: "gitlab-issues",
        service: "gitlab",
        description: "GitLab Issues",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "variant",
                required: true,
                example: "",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
            Param {
                name: "labels",
                required: false,
                example: "test,failure::new",
            },
        ],
        numeric: false,
        resolve: resolve_issues,
    },
    PresetMeta {
        preset: "gitlab-languages-count",
        service: "gitlab",
        description: "GitLab Language Count",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
        ],
        numeric: true,
        resolve: resolve_languages_count,
    },
    PresetMeta {
        preset: "gitlab-last-commit",
        service: "gitlab",
        description: "GitLab Last Commit",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
            Param {
                name: "ref",
                required: false,
                example: "master",
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
        preset: "gitlab-license",
        service: "gitlab",
        description: "GitLab License",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
        ],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "gitlab-pipeline-coverage",
        service: "gitlab",
        description: "Gitlab Code Coverage",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "branch",
                required: true,
                example: "master",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
            Param {
                name: "job-name",
                required: false,
                example: "jest-integration",
            },
        ],
        numeric: false,
        resolve: resolve_pipeline_coverage,
    },
    PresetMeta {
        preset: "gitlab-pipeline-status",
        service: "gitlab",
        description: "Gitlab Pipeline Status",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_pipeline_status,
    },
    PresetMeta {
        preset: "gitlab-release",
        service: "gitlab",
        description: "GitLab Release",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
            Param {
                name: "sort",
                required: false,
                example: "",
            },
            Param {
                name: "display-name",
                required: false,
                example: "",
            },
            Param {
                name: "date-order-by",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_release,
    },
    PresetMeta {
        preset: "gitlab-stars",
        service: "gitlab",
        description: "GitLab Stars",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
        ],
        numeric: true,
        resolve: resolve_stars,
    },
    PresetMeta {
        preset: "gitlab-tag",
        service: "gitlab",
        description: "GitLab Tag",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "shields-ops-group/tag-test",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
            Param {
                name: "sort",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_tag,
    },
    PresetMeta {
        preset: "gitlab-top-language",
        service: "gitlab",
        description: "GitLab Top Language",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "gitlab-org/gitlab",
            },
            Param {
                name: "gitlab-url",
                required: false,
                example: "https://gitlab.com",
            },
        ],
        numeric: false,
        resolve: resolve_top_language,
    },
];
