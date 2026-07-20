use super::meta::{Param, PresetMeta};
mod json;
mod regex;
mod toml;
mod xml;
mod yaml;

pub(crate) use json::resolve_json;
pub(crate) use regex::resolve_regex;
pub(crate) use toml::resolve_toml;
pub(crate) use xml::resolve_xml;
pub(crate) use yaml::resolve_yaml;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "dynamic-json",
        service: "dynamic",
        description: "Dynamic JSON Badge",
        params: &[
            Param {
                name: "url",
                required: true,
                example: "https://github.com/badges/shields/raw/master/package.json",
            },
            Param {
                name: "query",
                required: true,
                example: "$.name",
            },
        ],
        numeric: false,
        resolve: resolve_json,
    },
    PresetMeta {
        preset: "dynamic-regex",
        service: "dynamic",
        description: "Dynamic Regex Badge",
        params: &[
            Param {
                name: "url",
                required: true,
                example: "https://raw.githubusercontent.com/badges/shields/refs/heads/master/README.md",
            },
            Param {
                name: "search",
                required: true,
                example: "Every (.\\*?) it serves (?<amount>.\\*?) images",
            },
        ],
        numeric: false,
        resolve: resolve_regex,
    },
    PresetMeta {
        preset: "dynamic-toml",
        service: "dynamic",
        description: "Dynamic TOML Badge",
        params: &[
            Param {
                name: "url",
                required: true,
                example: "https://raw.githubusercontent.com/squirrelchat/smol-toml/mistress/bench/testfiles/toml-spec-example.toml",
            },
            Param {
                name: "query",
                required: true,
                example: "$.title",
            },
        ],
        numeric: false,
        resolve: resolve_toml,
    },
    PresetMeta {
        preset: "dynamic-xml",
        service: "dynamic",
        description: "Dynamic XML Badge",
        params: &[
            Param {
                name: "url",
                required: true,
                example: "https://httpbin.org/xml",
            },
            Param {
                name: "query",
                required: true,
                example: "//slideshow/slide[1]/title",
            },
        ],
        numeric: false,
        resolve: resolve_xml,
    },
    PresetMeta {
        preset: "dynamic-yaml",
        service: "dynamic",
        description: "Dynamic YAML Badge",
        params: &[
            Param {
                name: "url",
                required: true,
                example: "https://raw.githubusercontent.com/badges/shields/master/.github/dependabot.yml",
            },
            Param {
                name: "query",
                required: true,
                example: "$.version",
            },
        ],
        numeric: false,
        resolve: resolve_yaml,
    },
];
