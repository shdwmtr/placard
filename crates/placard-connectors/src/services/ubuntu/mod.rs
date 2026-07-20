use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod ubuntu;

pub(crate) use ubuntu::resolve_ubuntu;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "ubuntu",
    service: "ubuntu",
    description: "Ubuntu Package Version (for series)",
    params: &[
        Param {
            name: "package-name",
            required: true,
            example: "ubuntu-wallpapers",
        },
        Param {
            name: "series",
            required: false,
            example: "bionic",
        },
    ],
    numeric: false,
    resolve: resolve_ubuntu,
}];
