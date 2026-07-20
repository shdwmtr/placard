use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod scan;

pub(crate) use scan::resolve_scan;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "coverity-scan",
    service: "coverity",
    description: "Coverity Scan",
    params: &[Param {
        name: "project-id",
        required: true,
        example: "3997",
    }],
    numeric: false,
    resolve: resolve_scan,
}];
