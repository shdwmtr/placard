use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod component_license;
mod entities;
mod project_translated_percentage;
mod user_statistic;

pub(crate) use component_license::resolve_component_license;
pub(crate) use entities::resolve_entities;
pub(crate) use project_translated_percentage::resolve_project_translated_percentage;
pub(crate) use user_statistic::resolve_user_statistic;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "weblate-component-license",
        service: "weblate",
        description: "Weblate component license",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "godot-engine",
            },
            Param {
                name: "component",
                required: true,
                example: "godot",
            },
            Param {
                name: "server",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_component_license,
    },
    PresetMeta {
        preset: "weblate-entities",
        service: "weblate",
        description: "Weblate entities",
        params: &[
            Param {
                name: "type",
                required: true,
                example: "",
            },
            Param {
                name: "server",
                required: false,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_entities,
    },
    PresetMeta {
        preset: "weblate-project-translated-percentage",
        service: "weblate",
        description: "Weblate project translated",
        params: &[
            Param {
                name: "project",
                required: true,
                example: "godot-engine",
            },
            Param {
                name: "server",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_project_translated_percentage,
    },
    PresetMeta {
        preset: "weblate-user-statistic",
        service: "weblate",
        description: "Weblate user statistic",
        params: &[
            Param {
                name: "statistic",
                required: true,
                example: "",
            },
            Param {
                name: "user",
                required: true,
                example: "nijel",
            },
            Param {
                name: "server",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_user_statistic,
    },
];
