use super::meta::{Param, PresetMeta};
mod module_downloads;
mod module_endorsement;
mod module_feedback;
mod module_pdk_version;
mod module_quality_score;
mod module_version;
mod user_module_count;
mod user_release_count;

pub(crate) use module_downloads::resolve_module_downloads;
pub(crate) use module_endorsement::resolve_module_endorsement;
pub(crate) use module_feedback::resolve_module_feedback;
pub(crate) use module_pdk_version::resolve_module_pdk_version;
pub(crate) use module_quality_score::resolve_module_quality_score;
pub(crate) use module_version::resolve_module_version;
pub(crate) use user_module_count::resolve_user_module_count;
pub(crate) use user_release_count::resolve_user_release_count;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "puppetforge-module-downloads",
        service: "puppetforge",
        description: "Puppet Forge downloads",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "camptocamp",
            },
            Param {
                name: "module-name",
                required: true,
                example: "openldap",
            },
        ],
        numeric: true,
        resolve: resolve_module_downloads,
    },
    PresetMeta {
        preset: "puppetforge-module-endorsement",
        service: "puppetforge",
        description: "Puppet Forge endorsement",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "camptocamp",
            },
            Param {
                name: "module-name",
                required: true,
                example: "openssl",
            },
        ],
        numeric: false,
        resolve: resolve_module_endorsement,
    },
    PresetMeta {
        preset: "puppetforge-module-feedback",
        service: "puppetforge",
        description: "Puppet Forge feedback score",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "camptocamp",
            },
            Param {
                name: "module-name",
                required: true,
                example: "openssl",
            },
        ],
        numeric: false,
        resolve: resolve_module_feedback,
    },
    PresetMeta {
        preset: "puppetforge-module-pdk-version",
        service: "puppetforge",
        description: "Puppet Forge - PDK version",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "tragiccode",
            },
            Param {
                name: "module-name",
                required: true,
                example: "azure_key_vault",
            },
        ],
        numeric: false,
        resolve: resolve_module_pdk_version,
    },
    PresetMeta {
        preset: "puppetforge-module-quality-score",
        service: "puppetforge",
        description: "Puppet Forge quality score",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "camptocamp",
            },
            Param {
                name: "module-name",
                required: true,
                example: "openssl",
            },
        ],
        numeric: false,
        resolve: resolve_module_quality_score,
    },
    PresetMeta {
        preset: "puppetforge-module-version",
        service: "puppetforge",
        description: "Puppet Forge version",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "vStone",
            },
            Param {
                name: "module-name",
                required: true,
                example: "percona",
            },
        ],
        numeric: false,
        resolve: resolve_module_version,
    },
    PresetMeta {
        preset: "puppetforge-user-module-count",
        service: "puppetforge",
        description: "Puppet Forge modules by user",
        params: &[Param {
            name: "user",
            required: true,
            example: "camptocamp",
        }],
        numeric: true,
        resolve: resolve_user_module_count,
    },
    PresetMeta {
        preset: "puppetforge-user-release-count",
        service: "puppetforge",
        description: "Puppet Forge releases by user",
        params: &[Param {
            name: "user",
            required: true,
            example: "camptocamp",
        }],
        numeric: true,
        resolve: resolve_user_release_count,
    },
];
