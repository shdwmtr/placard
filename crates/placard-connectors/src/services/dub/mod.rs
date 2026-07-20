use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod download;
mod license;
mod score;
mod version;

pub(crate) use download::resolve_download;
pub(crate) use license::resolve_license;
pub(crate) use score::resolve_score;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "dub-download",
        service: "dub",
        description: "DUB Downloads",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "",
            },
            Param {
                name: "interval",
                required: true,
                example: "",
            },
            Param {
                name: "version",
                required: false,
                example: "0.8.4",
            },
        ],
        numeric: true,
        resolve: resolve_download,
    },
    PresetMeta {
        preset: "dub-license",
        service: "dub",
        description: "DUB License",
        params: &[Param {
            name: "package",
            required: true,
            example: "vibe-d",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "dub-score",
        service: "dub",
        description: "DUB Score",
        params: &[Param {
            name: "package",
            required: true,
            example: "vibe-d",
        }],
        numeric: true,
        resolve: resolve_score,
    },
    PresetMeta {
        preset: "dub-version",
        service: "dub",
        description: "DUB Version",
        params: &[Param {
            name: "package",
            required: true,
            example: "vibe-d",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
