use super::meta::{Param, PresetMeta};
mod points;

pub(crate) use points::resolve_points;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "freecodecamp-points",
    service: "freecodecamp",
    description: "freeCodeCamp points",
    params: &[Param {
        name: "username",
        required: true,
        example: "qapaloma",
    }],
    numeric: true,
    resolve: resolve_points,
}];
