use super::meta::{Param, PresetMeta};
mod monthlyquestions;
mod reputation;
mod taginfo;

pub(crate) use monthlyquestions::resolve_monthlyquestions;
pub(crate) use reputation::resolve_reputation;
pub(crate) use taginfo::resolve_taginfo;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "stackexchange-monthlyquestions",
        service: "stackexchange",
        description: "Stack Exchange monthly questions",
        params: &[
            Param {
                name: "stackexchangesite",
                required: true,
                example: "stackoverflow",
            },
            Param {
                name: "query",
                required: true,
                example: "javascript",
            },
        ],
        numeric: true,
        resolve: resolve_monthlyquestions,
    },
    PresetMeta {
        preset: "stackexchange-reputation",
        service: "stackexchange",
        description: "Stack Exchange reputation",
        params: &[
            Param {
                name: "stackexchangesite",
                required: true,
                example: "stackoverflow",
            },
            Param {
                name: "query",
                required: true,
                example: "123",
            },
        ],
        numeric: true,
        resolve: resolve_reputation,
    },
    PresetMeta {
        preset: "stackexchange-taginfo",
        service: "stackexchange",
        description: "Stack Exchange questions",
        params: &[
            Param {
                name: "stackexchangesite",
                required: true,
                example: "stackoverflow",
            },
            Param {
                name: "query",
                required: true,
                example: "gson",
            },
        ],
        numeric: true,
        resolve: resolve_taginfo,
    },
];
