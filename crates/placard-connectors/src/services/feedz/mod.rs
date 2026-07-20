use super::meta::{Param, PresetMeta};
mod feedz;

pub(crate) use feedz::resolve_feedz;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "feedz",
    service: "feedz",
    description: "Feedz Version",
    params: &[
        Param {
            name: "organization",
            required: true,
            example: "shieldstests",
        },
        Param {
            name: "repository",
            required: true,
            example: "mongodb",
        },
        Param {
            name: "package-name",
            required: true,
            example: "MongoDB.Driver.Core",
        },
        Param {
            name: "variant",
            required: false,
            example: "",
        },
    ],
    numeric: false,
    resolve: resolve_feedz,
}];
