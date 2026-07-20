use super::meta::{Param, PresetMeta};
mod dependencies;
mod dependent_repos;
mod dependents;
mod sourcerank;

pub(crate) use dependencies::resolve_dependencies;
pub(crate) use dependent_repos::resolve_dependent_repos;
pub(crate) use dependents::resolve_dependents;
pub(crate) use sourcerank::resolve_sourcerank;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "librariesio-dependencies",
        service: "librariesio",
        description: "Libraries.io dependency status for latest release",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "phoenixframework",
            },
            Param {
                name: "repo",
                required: true,
                example: "phoenix",
            },
        ],
        numeric: false,
        resolve: resolve_dependencies,
    },
    PresetMeta {
        preset: "librariesio-dependent-repos",
        service: "librariesio",
        description: "Dependent repos (via libraries.io)",
        params: &[
            Param {
                name: "platform",
                required: true,
                example: "npm",
            },
            Param {
                name: "package-name",
                required: true,
                example: "got",
            },
            Param {
                name: "scope",
                required: false,
                example: "@babel",
            },
        ],
        numeric: true,
        resolve: resolve_dependent_repos,
    },
    PresetMeta {
        preset: "librariesio-dependents",
        service: "librariesio",
        description: "Dependents (via libraries.io)",
        params: &[
            Param {
                name: "platform",
                required: true,
                example: "npm",
            },
            Param {
                name: "package-name",
                required: true,
                example: "got",
            },
            Param {
                name: "scope",
                required: false,
                example: "@babel",
            },
        ],
        numeric: true,
        resolve: resolve_dependents,
    },
    PresetMeta {
        preset: "librariesio-sourcerank",
        service: "librariesio",
        description: "Libraries.io SourceRank",
        params: &[
            Param {
                name: "platform",
                required: true,
                example: "npm",
            },
            Param {
                name: "package-name",
                required: true,
                example: "got",
            },
            Param {
                name: "scope",
                required: false,
                example: "@babel",
            },
        ],
        numeric: true,
        resolve: resolve_sourcerank,
    },
];
