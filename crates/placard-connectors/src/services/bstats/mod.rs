use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod players;
mod servers;

pub(crate) use players::resolve_players;
pub(crate) use servers::resolve_servers;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "bstats-players",
        service: "bstats",
        description: "bStats Players",
        params: &[Param {
            name: "pluginid",
            required: true,
            example: "1",
        }],
        numeric: true,
        resolve: resolve_players,
    },
    PresetMeta {
        preset: "bstats-servers",
        service: "bstats",
        description: "bStats Servers",
        params: &[Param {
            name: "pluginid",
            required: true,
            example: "1",
        }],
        numeric: true,
        resolve: resolve_servers,
    },
];
