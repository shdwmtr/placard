use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod elm_package;

pub(crate) use elm_package::resolve_elm_package;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "elm-package",
    service: "elm_package",
    description: "Elm package",
    params: &[
        Param {
            name: "user",
            required: true,
            example: "elm",
        },
        Param {
            name: "package",
            required: true,
            example: "core",
        },
    ],
    numeric: false,
    resolve: resolve_elm_package,
}];
