use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod lemmy;

pub(crate) use lemmy::resolve_lemmy;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "lemmy",
    service: "lemmy",
    description: "Lemmy",
    params: &[Param {
        name: "community",
        required: true,
        example: "asklemmy@lemmy.ml",
    }],
    numeric: true,
    resolve: resolve_lemmy,
}];
