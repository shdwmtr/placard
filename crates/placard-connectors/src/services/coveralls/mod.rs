use super::meta::{Param, PresetMeta};
mod coveralls;

pub(crate) use coveralls::resolve_coveralls;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "coveralls",
    service: "coveralls",
    description: "Coveralls",
    params: &[
        Param {
            name: "user",
            required: true,
            example: "jekyll",
        },
        Param {
            name: "repo",
            required: true,
            example: "jekyll",
        },
        Param {
            name: "vcs-type",
            required: false,
            example: "",
        },
        Param {
            name: "branch",
            required: false,
            example: "master",
        },
    ],
    numeric: false,
    resolve: resolve_coveralls,
}];
