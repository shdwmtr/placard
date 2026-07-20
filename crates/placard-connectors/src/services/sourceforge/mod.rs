use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod commit_count;
mod contributors;
mod downloads;
mod languages;
mod last_commit;
mod open_tickets;
mod platform;
mod translations;

pub(crate) use commit_count::resolve_commit_count;
pub(crate) use contributors::resolve_contributors;
pub(crate) use downloads::resolve_downloads;
pub(crate) use languages::resolve_languages;
pub(crate) use last_commit::resolve_last_commit;
pub(crate) use open_tickets::resolve_open_tickets;
pub(crate) use platform::resolve_platform;
pub(crate) use translations::resolve_translations;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "sourceforge-commit-count",
        service: "sourceforge",
        description: "SourceForge Commit Count",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "guitarix",
            },
            Param {
                name: "repo",
                required: true,
                example: "git",
            },
        ],
        numeric: true,
        resolve: resolve_commit_count,
    },
    PresetMeta {
        preset: "sourceforge-contributors",
        service: "sourceforge",
        description: "SourceForge Contributors",
        params: &[Param {
            name: "project",
            required: true,
            example: "guitarix",
        }],
        numeric: true,
        resolve: resolve_contributors,
    },
    PresetMeta {
        preset: "sourceforge-downloads",
        service: "sourceforge",
        description: "SourceForge Downloads",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "sevenzip",
            },
            Param {
                name: "interval",
                required: true,
                example: "",
            },
            Param {
                name: "folder",
                required: false,
                example: "stendhal",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "sourceforge-languages",
        service: "sourceforge",
        description: "SourceForge Languages",
        params: &[Param {
            name: "project",
            required: true,
            example: "mingw",
        }],
        numeric: true,
        resolve: resolve_languages,
    },
    PresetMeta {
        preset: "sourceforge-last-commit",
        service: "sourceforge",
        description: "SourceForge Last Commit",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "guitarix",
            },
            Param {
                name: "repo",
                required: true,
                example: "git",
            },
        ],
        numeric: false,
        resolve: resolve_last_commit,
    },
    PresetMeta {
        preset: "sourceforge-open-tickets",
        service: "sourceforge",
        description: "Sourceforge Open Tickets",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "sevenzip",
            },
            Param {
                name: "type",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_open_tickets,
    },
    PresetMeta {
        preset: "sourceforge-platform",
        service: "sourceforge",
        description: "SourceForge Platform",
        params: &[Param {
            name: "project",
            required: true,
            example: "guitarix",
        }],
        numeric: false,
        resolve: resolve_platform,
    },
    PresetMeta {
        preset: "sourceforge-translations",
        service: "sourceforge",
        description: "SourceForge Translations",
        params: &[Param {
            name: "project",
            required: true,
            example: "guitarix",
        }],
        numeric: true,
        resolve: resolve_translations,
    },
];
