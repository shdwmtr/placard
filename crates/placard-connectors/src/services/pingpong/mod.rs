use super::meta::{Param, PresetMeta};
mod status;
mod uptime;

pub(crate) use status::resolve_status;
pub(crate) use uptime::resolve_uptime;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "pingpong-status",
        service: "pingpong",
        description: "PingPong status",
        params: &[Param {
            name: "api-key",
            required: true,
            example: "sp_2e80bc00b6054faeb2b87e2464be337e",
        }],
        numeric: false,
        resolve: resolve_status,
    },
    PresetMeta {
        preset: "pingpong-uptime",
        service: "pingpong",
        description: "PingPong uptime (last 30 days)",
        params: &[Param {
            name: "api-key",
            required: true,
            example: "sp_2e80bc00b6054faeb2b87e2464be337e",
        }],
        numeric: false,
        resolve: resolve_uptime,
    },
];
