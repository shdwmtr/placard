use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod btc;
mod pgp;
mod xlm;
mod zec;

pub(crate) use btc::resolve_btc;
pub(crate) use pgp::resolve_pgp;
pub(crate) use xlm::resolve_xlm;
pub(crate) use zec::resolve_zec;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "keybase-btc",
        service: "keybase",
        description: "Keybase BTC",
        params: &[Param {
            name: "username",
            required: true,
            example: "skyplabs",
        }],
        numeric: false,
        resolve: resolve_btc,
    },
    PresetMeta {
        preset: "keybase-pgp",
        service: "keybase",
        description: "Keybase PGP",
        params: &[Param {
            name: "username",
            required: true,
            example: "skyplabs",
        }],
        numeric: false,
        resolve: resolve_pgp,
    },
    PresetMeta {
        preset: "keybase-xlm",
        service: "keybase",
        description: "Keybase XLM",
        params: &[Param {
            name: "username",
            required: true,
            example: "skyplabs",
        }],
        numeric: false,
        resolve: resolve_xlm,
    },
    PresetMeta {
        preset: "keybase-zec",
        service: "keybase",
        description: "Keybase ZEC",
        params: &[Param {
            name: "username",
            required: true,
            example: "skyplabs",
        }],
        numeric: false,
        resolve: resolve_zec,
    },
];
