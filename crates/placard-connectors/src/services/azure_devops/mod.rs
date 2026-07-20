use super::meta::{Param, PresetMeta};
mod build;
mod coverage;
mod release;
mod tests;

pub(crate) use build::resolve_build;
pub(crate) use coverage::resolve_coverage;
pub(crate) use release::resolve_release;
pub(crate) use tests::resolve_tests;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "azure-devops-build",
        service: "azure_devops",
        description: "Azure DevOps builds",
        params: &[
            Param {
                name: "organization",
                required: true,
                example: "totodem",
            },
            Param {
                name: "project-id",
                required: true,
                example: "8cf3ec0e-d0c2-4fcd-8206-ad204f254a96",
            },
            Param {
                name: "definition-id",
                required: true,
                example: "2",
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
        preset: "azure-devops-coverage",
        service: "azure_devops",
        description: "Azure DevOps coverage",
        params: &[
            Param {
                name: "organization",
                required: true,
                example: "swellaby",
            },
            Param {
                name: "project",
                required: true,
                example: "opensource",
            },
            Param {
                name: "definition-id",
                required: true,
                example: "25",
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
        preset: "azure-devops-release",
        service: "azure_devops",
        description: "Azure DevOps releases",
        params: &[
            Param {
                name: "organization",
                required: true,
                example: "totodem",
            },
            Param {
                name: "project-id",
                required: true,
                example: "8cf3ec0e-d0c2-4fcd-8206-ad204f254a96",
            },
            Param {
                name: "definition-id",
                required: true,
                example: "1",
            },
            Param {
                name: "environment-id",
                required: true,
                example: "1",
            },
        ],
        numeric: false,
        resolve: resolve_release,
    },
    PresetMeta {
        preset: "azure-devops-tests",
        service: "azure_devops",
        description: "Azure DevOps tests",
        params: &[
            Param {
                name: "organization",
                required: true,
                example: "azuredevops-powershell",
            },
            Param {
                name: "project",
                required: true,
                example: "azuredevops-powershell",
            },
            Param {
                name: "definition-id",
                required: true,
                example: "1",
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
