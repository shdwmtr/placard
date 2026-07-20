use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod build;

pub(crate) use build::resolve_build;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "travis-build",
    service: "travis",
    description: "Travis (.com)",
    params: &[
        Param {
            name: "user",
            required: true,
            example: "ivandelabeldad",
        },
        Param {
            name: "repo",
            required: true,
            example: "rackian-gateway",
        },
        Param {
            name: "branch",
            required: false,
            example: "master",
        },
    ],
    numeric: false,
    resolve: resolve_build,
}];
