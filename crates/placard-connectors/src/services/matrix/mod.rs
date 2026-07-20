use super::meta::{Param, PresetMeta};
mod matrix;

pub(crate) use matrix::resolve_matrix;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "matrix",
    service: "matrix",
    description: "Matrix",
    params: &[
        Param {
            name: "room-alias",
            required: true,
            example: "twim:matrix.org",
        },
        Param {
            name: "server_fqdn",
            required: false,
            example: "matrix.org",
        },
    ],
    numeric: true,
    resolve: resolve_matrix,
}];
