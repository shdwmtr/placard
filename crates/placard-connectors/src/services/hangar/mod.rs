use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod stars;
mod views;
mod watchers;

pub(crate) use downloads::resolve_downloads;
pub(crate) use stars::resolve_stars;
pub(crate) use views::resolve_views;
pub(crate) use watchers::resolve_watchers;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "hangar-downloads",
        service: "hangar",
        description: "Hangar Downloads",
        params: &[Param {
            name: "slug",
            required: true,
            example: "Essentials",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "hangar-stars",
        service: "hangar",
        description: "Hangar Stars",
        params: &[Param {
            name: "slug",
            required: true,
            example: "Essentials",
        }],
        numeric: true,
        resolve: resolve_stars,
    },
    PresetMeta {
        preset: "hangar-views",
        service: "hangar",
        description: "Hangar Views",
        params: &[Param {
            name: "slug",
            required: true,
            example: "Essentials",
        }],
        numeric: true,
        resolve: resolve_views,
    },
    PresetMeta {
        preset: "hangar-watchers",
        service: "hangar",
        description: "Hangar Watchers",
        params: &[Param {
            name: "slug",
            required: true,
            example: "Essentials",
        }],
        numeric: true,
        resolve: resolve_watchers,
    },
];
