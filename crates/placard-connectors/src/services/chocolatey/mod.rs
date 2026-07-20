use super::meta::{Param, PresetMeta};
mod downloads;
mod version;

pub(crate) use downloads::resolve_dt;
pub(crate) use version::resolve_v;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "chocolatey-dt",
        service: "chocolatey",
        description: "",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_dt,
    },
    PresetMeta {
        preset: "chocolatey-v",
        service: "chocolatey",
        description: "",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "",
            },
            Param {
                name: "include_prereleases",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_v,
    },
];
