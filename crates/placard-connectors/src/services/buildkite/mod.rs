use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod buildkite;

pub(crate) use buildkite::resolve_buildkite;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "buildkite",
    service: "buildkite",
    description: "Buildkite",
    params: &[
        Param {
            name: "identifier",
            required: true,
            example: "3826789cf8890b426057e6fe1c4e683bdf04fa24d498885489",
        },
        Param {
            name: "branch",
            required: false,
            example: "master",
        },
    ],
    numeric: false,
    resolve: resolve_buildkite,
}];
