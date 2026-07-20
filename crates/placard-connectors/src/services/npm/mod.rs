use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod collaborators;
mod dependency_version;
mod downloads;
mod last_update;
mod license;
mod type_definitions;
mod unpacked_size;
mod version;

pub(crate) use collaborators::resolve_collaborators;
pub(crate) use dependency_version::resolve_dependency_version;
pub(crate) use downloads::resolve_downloads;
pub(crate) use last_update::resolve_last_update;
pub(crate) use license::resolve_license;
pub(crate) use type_definitions::resolve_type_definitions;
pub(crate) use unpacked_size::resolve_unpacked_size;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "npm-collaborators",
        service: "npm",
        description: "NPM Collaborators",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "prettier",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: true,
        resolve: resolve_collaborators,
    },
    PresetMeta {
        preset: "npm-dependency-version",
        service: "npm",
        description: "NPM (prod) Dependency Version",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "",
            },
            Param {
                name: "dependency",
                required: true,
                example: "simple-statistics",
            },
            Param {
                name: "kind",
                required: false,
                example: "",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: false,
        resolve: resolve_dependency_version,
    },
    PresetMeta {
        preset: "npm-downloads",
        service: "npm",
        description: "NPM Downloads",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "",
            },
            Param {
                name: "interval",
                required: false,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "npm-last-update",
        service: "npm",
        description: "NPM Last Update (with dist tag)",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "verdaccio",
            },
            Param {
                name: "tag",
                required: false,
                example: "next-8",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: false,
        resolve: resolve_last_update,
    },
    PresetMeta {
        preset: "npm-license",
        service: "npm",
        description: "NPM License",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "express",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "npm-type-definitions",
        service: "npm",
        description: "NPM Type Definitions",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "chalk",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: false,
        resolve: resolve_type_definitions,
    },
    PresetMeta {
        preset: "npm-unpacked-size",
        service: "npm",
        description: "NPM Unpacked Size",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "npm",
            },
            Param {
                name: "version",
                required: false,
                example: "4.18.2",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: true,
        resolve: resolve_unpacked_size,
    },
    PresetMeta {
        preset: "npm-version",
        service: "npm",
        description: "NPM Version",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "npm",
            },
            Param {
                name: "tag",
                required: false,
                example: "next-8",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
