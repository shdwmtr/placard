use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod all;
mod backers;
mod by_tier;
mod sponsors;

pub(crate) use all::resolve_all;
pub(crate) use backers::resolve_backers;
pub(crate) use by_tier::resolve_by_tier;
pub(crate) use sponsors::resolve_sponsors;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "opencollective-all",
        service: "opencollective",
        description: "Open Collective backers and sponsors",
        params: &[Param {
            name: "collective",
            required: true,
            example: "shields",
        }],
        numeric: true,
        resolve: resolve_all,
    },
    PresetMeta {
        preset: "opencollective-backers",
        service: "opencollective",
        description: "Open Collective backers",
        params: &[Param {
            name: "collective",
            required: true,
            example: "shields",
        }],
        numeric: true,
        resolve: resolve_backers,
    },
    PresetMeta {
        preset: "opencollective-by-tier",
        service: "opencollective",
        description: "Open Collective members by tier",
        params: &[
            Param {
                name: "collective",
                required: true,
                example: "shields",
            },
            Param {
                name: "tier-id",
                required: true,
                example: "2988",
            },
        ],
        numeric: true,
        resolve: resolve_by_tier,
    },
    PresetMeta {
        preset: "opencollective-sponsors",
        service: "opencollective",
        description: "Open Collective sponsors",
        params: &[Param {
            name: "collective",
            required: true,
            example: "shields",
        }],
        numeric: true,
        resolve: resolve_sponsors,
    },
];
