use super::meta::{Param, PresetMeta};
mod fedora;

pub(crate) use fedora::resolve_fedora;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "fedora",
    service: "fedora",
    description: "Fedora package (with branch)",
    params: &[
        Param {
            name: "package-name",
            required: true,
            example: "rpm",
        },
        Param {
            name: "branch",
            required: false,
            example: "rawhide",
        },
    ],
    numeric: false,
    resolve: resolve_fedora,
}];
