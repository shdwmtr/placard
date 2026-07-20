use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod status;
mod uptime;

pub(crate) use status::resolve_status;
pub(crate) use uptime::resolve_uptime;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "nodeping-status",
        service: "nodeping",
        description: "",
        params: &[
            Param {
                name: "check-uuid",
                required: true,
                example: "",
            },
            Param {
                name: "up_message",
                required: false,
                example: "",
            },
            Param {
                name: "down_message",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_status,
    },
    PresetMeta {
        preset: "nodeping-uptime",
        service: "nodeping",
        description: "",
        params: &[Param {
            name: "check-uuid",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_uptime,
    },
];
