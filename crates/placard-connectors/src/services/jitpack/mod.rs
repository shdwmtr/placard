use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod version;

pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "jitpack-version",
    service: "jitpack",
    description: "JitPack",
    params: &[
        Param {
            name: "group-id",
            required: true,
            example: "com.github.jitpack",
        },
        Param {
            name: "artifact-id",
            required: true,
            example: "maven-simple",
        },
    ],
    numeric: false,
    resolve: resolve_version,
}];
