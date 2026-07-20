use super::meta::{Param, PresetMeta};
mod downloads;
mod license;
mod rating;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use license::resolve_license;
pub(crate) use rating::resolve_rating;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "greasyfork-downloads",
        service: "greasyfork",
        description: "Greasy Fork Downloads",
        params: &[
            Param {
                name: "variant",
                required: true,
                example: "",
            },
            Param {
                name: "script-id",
                required: true,
                example: "406540",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "greasyfork-license",
        service: "greasyfork",
        description: "Greasy Fork License",
        params: &[Param {
            name: "script-id",
            required: true,
            example: "406540",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "greasyfork-rating",
        service: "greasyfork",
        description: "Greasy Fork Rating",
        params: &[Param {
            name: "script-id",
            required: true,
            example: "406540",
        }],
        numeric: false,
        resolve: resolve_rating,
    },
    PresetMeta {
        preset: "greasyfork-version",
        service: "greasyfork",
        description: "Greasy Fork Version",
        params: &[Param {
            name: "script-id",
            required: true,
            example: "406540",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
