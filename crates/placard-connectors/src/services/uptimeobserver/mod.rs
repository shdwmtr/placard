use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod ratio;
mod status;

pub(crate) use ratio::resolve_ratio;
pub(crate) use status::resolve_status;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "uptimeobserver-ratio",
        service: "uptimeobserver",
        description: "UptimeObserver uptime ratio (1 day)",
        params: &[
            Param {
                name: "monitor-key",
                required: true,
                example: "33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
            },
            Param {
                name: "period",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_ratio,
    },
    PresetMeta {
        preset: "uptimeobserver-status",
        service: "uptimeobserver",
        description: "UptimeObserver status",
        params: &[
            Param {
                name: "monitor-key",
                required: true,
                example: "33Zw1rnH6veb4OLcskqvj6g9Lj4tnyxZ41",
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
];
