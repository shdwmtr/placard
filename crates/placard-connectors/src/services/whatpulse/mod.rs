use super::meta::{Param, PresetMeta};
mod whatpulse;

pub(crate) use whatpulse::resolve_whatpulse;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "whatpulse",
    service: "whatpulse",
    description: "WhatPulse",
    params: &[
        Param {
            name: "metric",
            required: true,
            example: "",
        },
        Param {
            name: "user-type",
            required: true,
            example: "",
        },
        Param {
            name: "id",
            required: true,
            example: "179734",
        },
    ],
    numeric: false,
    resolve: resolve_whatpulse,
}];
