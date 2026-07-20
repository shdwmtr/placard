use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod spack;

pub(crate) use spack::resolve_spack;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "spack",
    service: "spack",
    description: "Spack",
    params: &[Param {
        name: "package-name",
        required: true,
        example: "adios2",
    }],
    numeric: false,
    resolve: resolve_spack,
}];
