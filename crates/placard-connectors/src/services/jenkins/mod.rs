use super::meta::{Param, PresetMeta};
mod build;
mod coverage;
mod plugin_installs;
mod plugin_version;
mod tests;

pub(crate) use build::resolve_build;
pub(crate) use coverage::resolve_coverage;
pub(crate) use plugin_installs::resolve_plugin_installs;
pub(crate) use plugin_version::resolve_plugin_version;
pub(crate) use tests::resolve_tests;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "jenkins-build",
        service: "jenkins",
        description: "Jenkins Build",
        params: &[Param {
            name: "job-url",
            required: true,
            example: "https://ci.eclipse.org/jgit/job/jgit",
        }],
        numeric: false,
        resolve: resolve_build,
    },
    PresetMeta {
        preset: "jenkins-coverage",
        service: "jenkins",
        description: "Jenkins Coverage",
        params: &[Param {
            name: "job-url",
            required: true,
            example: "https://jenkins.mm12.xyz/jenkins/job/nmfu/job/master",
        }],
        numeric: false,
        resolve: resolve_coverage,
    },
    PresetMeta {
        preset: "jenkins-plugin-installs",
        service: "jenkins",
        description: "Jenkins Plugin installs",
        params: &[Param {
            name: "plugin",
            required: true,
            example: "view-job-filters",
        }],
        numeric: true,
        resolve: resolve_plugin_installs,
    },
    PresetMeta {
        preset: "jenkins-plugin-version",
        service: "jenkins",
        description: "Jenkins Plugin Version",
        params: &[Param {
            name: "plugin",
            required: true,
            example: "blueocean",
        }],
        numeric: false,
        resolve: resolve_plugin_version,
    },
    PresetMeta {
        preset: "jenkins-tests",
        service: "jenkins",
        description: "Jenkins Tests",
        params: &[Param {
            name: "job-url",
            required: true,
            example: "https://ci.eclipse.org/jgit/job/jgit",
        }],
        numeric: false,
        resolve: resolve_tests,
    },
];
