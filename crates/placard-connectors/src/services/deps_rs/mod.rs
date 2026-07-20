use super::meta::{Param, PresetMeta};
#[path = "crate.rs"]
mod krate;
mod repo;

pub(crate) use krate::resolve_crate;
pub(crate) use repo::resolve_repo;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "deps-rs-crate",
        service: "deps_rs",
        description: "Deps.rs Crate Dependencies (latest)",
        params: &[
            Param {
                name: "crate",
                required: true,
                example: "syn",
            },
            Param {
                name: "version",
                required: true,
                example: "2.0.101",
            },
        ],
        numeric: false,
        resolve: resolve_crate,
    },
    PresetMeta {
        preset: "deps-rs-repo",
        service: "deps_rs",
        description: "Deps.rs Repository Dependencies",
        params: &[
            Param {
                name: "site",
                required: true,
                example: "",
            },
            Param {
                name: "user",
                required: true,
                example: "dtolnay",
            },
            Param {
                name: "repo",
                required: true,
                example: "syn",
            },
        ],
        numeric: false,
        resolve: resolve_repo,
    },
];
