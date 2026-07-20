use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod current;
mod lts;

pub(crate) use current::resolve_current;
pub(crate) use lts::resolve_lts;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "node-current",
        service: "node",
        description: "Node Current",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "passport",
            },
            Param {
                name: "tag",
                required: false,
                example: "latest",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: false,
        resolve: resolve_current,
    },
    PresetMeta {
        preset: "node-lts",
        service: "node",
        description: "Node LTS",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "passport",
            },
            Param {
                name: "tag",
                required: false,
                example: "latest",
            },
            Param {
                name: "registry_uri",
                required: false,
                example: "https://registry.npmjs.com",
            },
        ],
        numeric: false,
        resolve: resolve_lts,
    },
];
