use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;

pub(crate) use downloads::resolve_downloads;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "myget-downloads",
    service: "myget",
    description: "",
    params: &[
        Param {
            name: "feed",
            required: true,
            example: "",
        },
        Param {
            name: "package-name",
            required: true,
            example: "",
        },
        Param {
            name: "tenant",
            required: false,
            example: "",
        },
    ],
    numeric: true,
    resolve: resolve_downloads,
}];
