use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod bitrise;

pub(crate) use bitrise::resolve_bitrise;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "bitrise",
    service: "bitrise",
    description: "Bitrise",
    params: &[
        Param {
            name: "app-id",
            required: true,
            example: "9fa2e96dc9458fbb",
        },
        Param {
            name: "token",
            required: true,
            example: "abc123def456",
        },
        Param {
            name: "branch",
            required: false,
            example: "master",
        },
    ],
    numeric: false,
    resolve: resolve_bitrise,
}];
