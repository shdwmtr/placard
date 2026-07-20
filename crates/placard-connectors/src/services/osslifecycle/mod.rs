use super::meta::{Param, PresetMeta};
mod osslifecycle;
mod redirector;

pub(crate) use osslifecycle::resolve_osslifecycle;
pub(crate) use redirector::resolve_redirector;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "osslifecycle",
        service: "osslifecycle",
        description: "OSS Lifecycle",
        params: &[Param {
            name: "file_url",
            required: true,
            example: "https://raw.githubusercontent.com/Netflix/aws-autoscaling/master/OSSMETADATA",
        }],
        numeric: false,
        resolve: resolve_osslifecycle,
    },
    PresetMeta {
        preset: "osslifecycle-redirector",
        service: "osslifecycle",
        description: "",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "",
            },
            Param {
                name: "repo",
                required: true,
                example: "",
            },
            Param {
                name: "branch",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_redirector,
    },
];
