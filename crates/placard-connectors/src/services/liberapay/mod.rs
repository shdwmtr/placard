use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod gives;
mod goal;
mod patrons;
mod receives;

pub(crate) use gives::resolve_gives;
pub(crate) use goal::resolve_goal;
pub(crate) use patrons::resolve_patrons;
pub(crate) use receives::resolve_receives;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "liberapay-gives",
        service: "liberapay",
        description: "Liberapay giving",
        params: &[Param {
            name: "entity",
            required: true,
            example: "Changaco",
        }],
        numeric: false,
        resolve: resolve_gives,
    },
    PresetMeta {
        preset: "liberapay-goal",
        service: "liberapay",
        description: "Liberapay goal progress",
        params: &[Param {
            name: "entity",
            required: true,
            example: "Changaco",
        }],
        numeric: false,
        resolve: resolve_goal,
    },
    PresetMeta {
        preset: "liberapay-patrons",
        service: "liberapay",
        description: "Liberapay patrons",
        params: &[Param {
            name: "entity",
            required: true,
            example: "Changaco",
        }],
        numeric: true,
        resolve: resolve_patrons,
    },
    PresetMeta {
        preset: "liberapay-receives",
        service: "liberapay",
        description: "Liberapay receiving",
        params: &[Param {
            name: "entity",
            required: true,
            example: "Changaco",
        }],
        numeric: false,
        resolve: resolve_receives,
    },
];
