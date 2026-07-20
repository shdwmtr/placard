use super::meta::{Param, PresetMeta};
mod dependents;
mod downloads;
mod license;
mod msrv;
mod size;
mod user_downloads;
mod version;

pub(crate) use dependents::resolve_dependents;
pub(crate) use downloads::resolve_downloads;
pub(crate) use license::resolve_license;
pub(crate) use msrv::resolve_msrv;
pub(crate) use size::resolve_size;
pub(crate) use user_downloads::resolve_user_downloads;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "crates-dependents",
        service: "crates",
        description: "Crates.io Dependents",
        params: &[Param {
            name: "crate",
            required: true,
            example: "tokio",
        }],
        numeric: true,
        resolve: resolve_dependents,
    },
    PresetMeta {
        preset: "crates-downloads",
        service: "crates",
        description: "Crates.io Total Downloads",
        params: &[
            Param {
                name: "crate",
                required: true,
                example: "rustc-serialize",
            },
            Param {
                name: "variant",
                required: false,
                example: "",
            },
            Param {
                name: "version",
                required: false,
                example: "0.3.24",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "crates-license",
        service: "crates",
        description: "Crates.io License",
        params: &[
            Param {
                name: "crate",
                required: true,
                example: "rustc-serialize",
            },
            Param {
                name: "version",
                required: false,
                example: "0.3.24",
            },
        ],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "crates-msrv",
        service: "crates",
        description: "Crates.io MSRV",
        params: &[
            Param {
                name: "crate",
                required: true,
                example: "serde",
            },
            Param {
                name: "version",
                required: false,
                example: "1.0.194",
            },
        ],
        numeric: false,
        resolve: resolve_msrv,
    },
    PresetMeta {
        preset: "crates-size",
        service: "crates",
        description: "Crates.io Size",
        params: &[
            Param {
                name: "crate",
                required: true,
                example: "rustc-serialize",
            },
            Param {
                name: "version",
                required: false,
                example: "0.3.24",
            },
        ],
        numeric: true,
        resolve: resolve_size,
    },
    PresetMeta {
        preset: "crates-user-downloads",
        service: "crates",
        description: "Crates.io User Total Downloads",
        params: &[Param {
            name: "user-id",
            required: true,
            example: "",
        }],
        numeric: true,
        resolve: resolve_user_downloads,
    },
    PresetMeta {
        preset: "crates-version",
        service: "crates",
        description: "Crates.io Version",
        params: &[Param {
            name: "crate",
            required: true,
            example: "rustc-serialize",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
