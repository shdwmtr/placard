use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod last_update;

pub(crate) use last_update::resolve_last_update;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "maven-central-last-update",
    service: "maven_central",
    description: "Maven Central Last Update",
    params: &[
        Param {
            name: "group-id",
            required: true,
            example: "com.google.guava",
        },
        Param {
            name: "artifact-id",
            required: true,
            example: "guava",
        },
    ],
    numeric: false,
    resolve: resolve_last_update,
}];
