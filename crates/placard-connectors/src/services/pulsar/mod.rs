use super::meta::{Param, PresetMeta};
mod downloads;
mod stargazers;

pub(crate) use downloads::resolve_downloads;
pub(crate) use stargazers::resolve_stargazers;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "pulsar-downloads",
        service: "pulsar",
        description: "Pulsar Downloads",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "hey-pane",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "pulsar-stargazers",
        service: "pulsar",
        description: "Pulsar Stargazers",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "hey-pane",
        }],
        numeric: true,
        resolve: resolve_stargazers,
    },
];
