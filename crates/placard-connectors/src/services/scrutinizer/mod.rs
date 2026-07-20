use super::meta::{Param, PresetMeta};
mod build;
mod coverage;
mod quality;

pub(crate) use build::resolve_build;
pub(crate) use coverage::resolve_coverage;
pub(crate) use quality::resolve_quality;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "scrutinizer-build",
        service: "scrutinizer",
        description: "Scrutinizer build (GitHub/Bitbucket)",
        params: &[
            Param {
                name: "vcs",
                required: true,
                example: "filp",
            },
            Param {
                name: "user",
                required: true,
                example: "filp",
            },
            Param {
                name: "repo",
                required: true,
                example: "whoops",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_build,
    },
    PresetMeta {
        preset: "scrutinizer-coverage",
        service: "scrutinizer",
        description: "Scrutinizer coverage (GitHub/Bitbucket)",
        params: &[
            Param {
                name: "vcs",
                required: true,
                example: "filp",
            },
            Param {
                name: "user",
                required: true,
                example: "filp",
            },
            Param {
                name: "repo",
                required: true,
                example: "whoops",
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
        preset: "scrutinizer-quality",
        service: "scrutinizer",
        description: "Scrutinizer quality (GitHub/Bitbucket)",
        params: &[
            Param {
                name: "vcs",
                required: true,
                example: "filp",
            },
            Param {
                name: "user",
                required: true,
                example: "filp",
            },
            Param {
                name: "repo",
                required: true,
                example: "whoops",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: true,
        resolve: resolve_quality,
    },
];
