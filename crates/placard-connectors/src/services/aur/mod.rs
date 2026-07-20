use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod last_modified;
mod license;
mod maintainer;
mod popularity;
mod version;
mod votes;

pub(crate) use last_modified::resolve_last_modified;
pub(crate) use license::resolve_license;
pub(crate) use maintainer::resolve_maintainer;
pub(crate) use popularity::resolve_popularity;
pub(crate) use version::resolve_version;
pub(crate) use votes::resolve_votes;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "aur-last-modified",
        service: "aur",
        description: "AUR Last Modified",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "google-chrome",
        }],
        numeric: true,
        resolve: resolve_last_modified,
    },
    PresetMeta {
        preset: "aur-license",
        service: "aur",
        description: "AUR License",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "android-studio",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "aur-maintainer",
        service: "aur",
        description: "AUR Maintainer",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "google-chrome",
        }],
        numeric: false,
        resolve: resolve_maintainer,
    },
    PresetMeta {
        preset: "aur-popularity",
        service: "aur",
        description: "AUR Popularity",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "dropbox",
        }],
        numeric: true,
        resolve: resolve_popularity,
    },
    PresetMeta {
        preset: "aur-version",
        service: "aur",
        description: "AUR Version",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "visual-studio-code-bin",
        }],
        numeric: false,
        resolve: resolve_version,
    },
    PresetMeta {
        preset: "aur-votes",
        service: "aur",
        description: "AUR Votes",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "dropbox",
        }],
        numeric: true,
        resolve: resolve_votes,
    },
];
