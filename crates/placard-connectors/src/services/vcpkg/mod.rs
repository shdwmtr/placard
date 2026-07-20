use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod version;

pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "vcpkg-version",
    service: "vcpkg",
    description: "Vcpkg Version",
    params: &[Param {
        name: "port-name",
        required: true,
        example: "entt",
    }],
    numeric: false,
    resolve: resolve_version,
}];
