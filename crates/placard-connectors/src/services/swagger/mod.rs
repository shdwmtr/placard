use super::meta::{Param, PresetMeta};
mod swagger;

pub(crate) use swagger::resolve_swagger;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "swagger",
    service: "swagger",
    description: "Swagger Validator",
    params: &[Param {
        name: "spec-url",
        required: true,
        example: "https://raw.githubusercontent.com/OAI/OpenAPI-Specification/c442afe06ec28443df0c69d01dc38c54968b246f/examples/v2.0/json/petstore-expanded.json",
    }],
    numeric: false,
    resolve: resolve_swagger,
}];
