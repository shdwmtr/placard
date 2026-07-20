use super::meta::{Param, PresetMeta};
mod endpoint;

pub(crate) use endpoint::resolve_endpoint;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "endpoint",
    service: "endpoint",
    description: "Endpoint Badge",
    params: &[Param {
        name: "url",
        required: true,
        example: "https://shields.redsparr0w.com/2473/monday",
    }],
    numeric: false,
    resolve: resolve_endpoint,
}];
