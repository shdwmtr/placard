use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod module_downloads;
mod provider_downloads;

pub(crate) use module_downloads::resolve_module_downloads;
pub(crate) use provider_downloads::resolve_provider_downloads;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "terraform-module-downloads",
        service: "terraform",
        description: "Terraform Module Downloads",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "hashicorp",
            },
            Param {
                name: "name",
                required: true,
                example: "consul",
            },
            Param {
                name: "provider",
                required: true,
                example: "aws",
            },
            Param {
                name: "interval",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_module_downloads,
    },
    PresetMeta {
        preset: "terraform-provider-downloads",
        service: "terraform",
        description: "Terraform Provider Downloads",
        params: &[
            Param {
                name: "provider-id",
                required: true,
                example: "",
            },
            Param {
                name: "interval",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_provider_downloads,
    },
];
