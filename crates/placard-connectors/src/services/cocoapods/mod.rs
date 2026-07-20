use super::meta::{Param, PresetMeta};
mod docs;
mod license;
mod platform;
mod version;

pub(crate) use docs::resolve_docs;
pub(crate) use license::resolve_license;
pub(crate) use platform::resolve_platform;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "cocoapods-docs",
        service: "cocoapods",
        description: "Cocoapods doc percentage",
        params: &[Param {
            name: "spec",
            required: true,
            example: "AFNetworking",
        }],
        numeric: true,
        resolve: resolve_docs,
    },
    PresetMeta {
        preset: "cocoapods-license",
        service: "cocoapods",
        description: "Cocoapods License",
        params: &[Param {
            name: "spec",
            required: true,
            example: "AFNetworking",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "cocoapods-platform",
        service: "cocoapods",
        description: "Cocoapods platforms",
        params: &[Param {
            name: "spec",
            required: true,
            example: "AFNetworking",
        }],
        numeric: false,
        resolve: resolve_platform,
    },
    PresetMeta {
        preset: "cocoapods-version",
        service: "cocoapods",
        description: "Cocoapods Version",
        params: &[Param {
            name: "spec",
            required: true,
            example: "AFNetworking",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
