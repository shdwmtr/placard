use super::meta::{Param, PresetMeta};
mod downloads;
mod owner;
mod rank;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use owner::resolve_owner;
pub(crate) use rank::resolve_rank;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "gem-downloads",
        service: "gem",
        description: "Gem Total Downloads",
        params: &[
            Param {
                name: "variant",
                required: true,
                example: "",
            },
            Param {
                name: "gem",
                required: true,
                example: "rails",
            },
            Param {
                name: "version",
                required: false,
                example: "4.1.0",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "gem-owner",
        service: "gem",
        description: "Gem Owner",
        params: &[Param {
            name: "user",
            required: true,
            example: "raphink",
        }],
        numeric: true,
        resolve: resolve_owner,
    },
    PresetMeta {
        preset: "gem-rank",
        service: "gem",
        description: "Gem download rank",
        params: &[
            Param {
                name: "period",
                required: true,
                example: "",
            },
            Param {
                name: "gem",
                required: true,
                example: "puppet",
            },
        ],
        numeric: true,
        resolve: resolve_rank,
    },
    PresetMeta {
        preset: "gem-version",
        service: "gem",
        description: "Gem Version",
        params: &[Param {
            name: "gem",
            required: true,
            example: "formatador",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
