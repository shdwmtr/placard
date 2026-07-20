use super::meta::{Param, PresetMeta};
mod build;
mod coverage;

pub(crate) use build::resolve_build;
pub(crate) use coverage::resolve_coverage;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "teamcity-build",
        service: "teamcity",
        description: "TeamCity Simple Build Status",
        params: &[
            Param {
                name: "build-id",
                required: true,
                example: "IntelliJIdeaCe_JavaDecompilerEngineTests",
            },
            Param {
                name: "server",
                required: false,
                example: "https://teamcity.jetbrains.com",
            },
            Param {
                name: "verbosity",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_build,
    },
    PresetMeta {
        preset: "teamcity-coverage",
        service: "teamcity",
        description: "TeamCity Coverage",
        params: &[
            Param {
                name: "build-id",
                required: true,
                example: "FileHelpersStable",
            },
            Param {
                name: "server",
                required: false,
                example: "https://teamcity.jetbrains.com",
            },
        ],
        numeric: false,
        resolve: resolve_coverage,
    },
];
