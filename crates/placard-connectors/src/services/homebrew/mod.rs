use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod cask_downloads;
mod cask_version;
mod formula_downloads;
mod formula_version;

pub(crate) use cask_downloads::resolve_cask_downloads;
pub(crate) use cask_version::resolve_cask_version;
pub(crate) use formula_downloads::resolve_formula_downloads;
pub(crate) use formula_version::resolve_formula_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "homebrew-cask-downloads",
        service: "homebrew",
        description: "Homebrew Cask Downloads",
        params: &[
            Param {
                name: "cask",
                required: true,
                example: "freetube",
            },
            Param {
                name: "interval",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_cask_downloads,
    },
    PresetMeta {
        preset: "homebrew-cask-version",
        service: "homebrew",
        description: "Homebrew Cask Version",
        params: &[Param {
            name: "cask",
            required: true,
            example: "iterm2",
        }],
        numeric: false,
        resolve: resolve_cask_version,
    },
    PresetMeta {
        preset: "homebrew-formula-downloads",
        service: "homebrew",
        description: "Homebrew Formula Downloads",
        params: &[
            Param {
                name: "formula",
                required: true,
                example: "cake",
            },
            Param {
                name: "interval",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_formula_downloads,
    },
    PresetMeta {
        preset: "homebrew-formula-version",
        service: "homebrew",
        description: "Homebrew Formula Version",
        params: &[Param {
            name: "formula",
            required: true,
            example: "cake",
        }],
        numeric: false,
        resolve: resolve_formula_version,
    },
];
