use super::meta::{Param, PresetMeta};
mod repositories;

pub(crate) use repositories::resolve_repositories;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "repology-repositories",
    service: "repology",
    description: "Repology - Repositories",
    params: &[Param {
        name: "project-name",
        required: true,
        example: "starship",
    }],
    numeric: true,
    resolve: resolve_repositories,
}];
