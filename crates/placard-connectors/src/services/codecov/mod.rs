use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod codecov;

pub(crate) use codecov::resolve_codecov;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "codecov",
    service: "codecov",
    description: "Codecov",
    params: &[
        Param {
            name: "vcs-name",
            required: true,
            example: "",
        },
        Param {
            name: "user",
            required: true,
            example: "codecov",
        },
        Param {
            name: "repo",
            required: true,
            example: "example-node",
        },
        Param {
            name: "branch",
            required: false,
            example: "master",
        },
        Param {
            name: "token",
            required: false,
            example: "a1b2c3d4e5",
        },
        Param {
            name: "flag",
            required: false,
            example: "flag_name",
        },
        Param {
            name: "component",
            required: false,
            example: "component_id_or_name",
        },
    ],
    numeric: false,
    resolve: resolve_codecov,
}];
