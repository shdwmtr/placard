use super::meta::{Param, PresetMeta};
mod reproducible_central;

pub(crate) use reproducible_central::resolve_reproducible_central;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "reproducible-central",
    service: "reproducible_central",
    description: "Reproducible Central Artifact",
    params: &[
        Param {
            name: "group-id",
            required: true,
            example: "org.apache.maven",
        },
        Param {
            name: "artifact-id",
            required: true,
            example: "maven-core",
        },
        Param {
            name: "version",
            required: true,
            example: "3.9.9",
        },
    ],
    numeric: false,
    resolve: resolve_reproducible_central,
}];
