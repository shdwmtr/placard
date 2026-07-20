use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod build;
mod job_build;
mod tests;

pub(crate) use build::resolve_build;
pub(crate) use job_build::resolve_job_build;
pub(crate) use tests::resolve_tests;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "appveyor-build",
        service: "appveyor",
        description: "AppVeyor Build",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "gruntjs",
            },
            Param {
                name: "repo",
                required: true,
                example: "grunt",
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
        preset: "appveyor-job-build",
        service: "appveyor",
        description: "AppVeyor Job",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "wpmgprostotema",
            },
            Param {
                name: "repo",
                required: true,
                example: "voicetranscoder",
            },
            Param {
                name: "job",
                required: true,
                example: "Linux",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_job_build,
    },
    PresetMeta {
        preset: "appveyor-tests",
        service: "appveyor",
        description: "AppVeyor tests",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "NZSmartie",
            },
            Param {
                name: "repo",
                required: true,
                example: "coap-net-iu0to",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_tests,
    },
];
