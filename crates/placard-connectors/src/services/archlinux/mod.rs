use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod archlinux;

pub(crate) use archlinux::resolve_archlinux;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "archlinux",
    service: "archlinux",
    description: "Arch Linux package",
    params: &[
        Param {
            name: "repository",
            required: true,
            example: "core",
        },
        Param {
            name: "architecture",
            required: true,
            example: "x86_64",
        },
        Param {
            name: "package-name",
            required: true,
            example: "pacman",
        },
    ],
    numeric: false,
    resolve: resolve_archlinux,
}];
