use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod activity;
mod downloads;
mod version;

pub(crate) use activity::resolve_activity;
pub(crate) use downloads::resolve_downloads;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "galaxytoolshed-activity",
        service: "galaxytoolshed",
        description: "Galaxy Toolshed - Created Date",
        params: &[
            Param {
                name: "repository",
                required: true,
                example: "sra_tools",
            },
            Param {
                name: "owner",
                required: true,
                example: "iuc",
            },
        ],
        numeric: false,
        resolve: resolve_activity,
    },
    PresetMeta {
        preset: "galaxytoolshed-downloads",
        service: "galaxytoolshed",
        description: "Galaxy Toolshed - Downloads",
        params: &[
            Param {
                name: "repository",
                required: true,
                example: "sra_tools",
            },
            Param {
                name: "owner",
                required: true,
                example: "iuc",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "galaxytoolshed-version",
        service: "galaxytoolshed",
        description: "Galaxy Toolshed - Repository Version",
        params: &[
            Param {
                name: "repository",
                required: true,
                example: "sra_tools",
            },
            Param {
                name: "owner",
                required: true,
                example: "iuc",
            },
            Param {
                name: "tool",
                required: false,
                example: "fastq_dump",
            },
            Param {
                name: "requirement",
                required: false,
                example: "perl",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
