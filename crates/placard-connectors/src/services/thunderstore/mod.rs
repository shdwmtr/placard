use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod likes;
mod version;

pub(crate) use downloads::resolve_downloads;
pub(crate) use likes::resolve_likes;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "thunderstore-downloads",
        service: "thunderstore",
        description: "Thunderstore Downloads",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "notnotnotswipez",
            },
            Param {
                name: "package-name",
                required: true,
                example: "MoreCompany",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "thunderstore-likes",
        service: "thunderstore",
        description: "Thunderstore Likes",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "notnotnotswipez",
            },
            Param {
                name: "package-name",
                required: true,
                example: "MoreCompany",
            },
        ],
        numeric: true,
        resolve: resolve_likes,
    },
    PresetMeta {
        preset: "thunderstore-version",
        service: "thunderstore",
        description: "Thunderstore Version",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "notnotnotswipez",
            },
            Param {
                name: "package-name",
                required: true,
                example: "MoreCompany",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
