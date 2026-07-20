use super::meta::{Param, PresetMeta};
mod circleci;

pub(crate) use circleci::resolve_circleci;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "circleci",
    service: "circleci",
    description: "CircleCI",
    params: &[
        Param {
            name: "vcs-type",
            required: true,
            example: "",
        },
        Param {
            name: "user",
            required: true,
            example: "RedSparr0w",
        },
        Param {
            name: "repo",
            required: true,
            example: "node-csgo-parser",
        },
        Param {
            name: "branch",
            required: false,
            example: "master",
        },
        Param {
            name: "token",
            required: false,
            example: "abc123def456",
        },
    ],
    numeric: false,
    resolve: resolve_circleci,
}];
