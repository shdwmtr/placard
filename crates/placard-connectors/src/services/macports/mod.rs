use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod macports;

pub(crate) use macports::resolve_macports;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "macports",
    service: "macports",
    description: "MacPorts Port Version",
    params: &[Param {
        name: "port-name",
        required: true,
        example: "git",
    }],
    numeric: false,
    resolve: resolve_macports,
}];
