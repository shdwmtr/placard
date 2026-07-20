use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod sourcegraph;

pub(crate) use sourcegraph::resolve_sourcegraph;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "sourcegraph",
    service: "sourcegraph",
    description: "Sourcegraph for Repo Reference Count",
    params: &[Param {
        name: "repo",
        required: true,
        example: "github.com/gorilla/mux",
    }],
    numeric: false,
    resolve: resolve_sourcegraph,
}];
