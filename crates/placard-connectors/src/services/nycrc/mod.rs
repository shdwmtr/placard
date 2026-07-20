use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod nycrc;

pub(crate) use nycrc::resolve_nycrc;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "nycrc",
    service: "nycrc",
    description: "nycrc config on GitHub",
    params: &[
        Param {
            name: "user",
            required: true,
            example: "yargs",
        },
        Param {
            name: "repo",
            required: true,
            example: "yargs",
        },
        Param {
            name: "config",
            required: false,
            example: ".nycrc",
        },
        Param {
            name: "preferred-threshold",
            required: false,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_nycrc,
}];
