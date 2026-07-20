use super::meta::{Param, PresetMeta};
mod version_from_toml;

pub(crate) use version_from_toml::resolve_version_from_toml;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "python-version-from-toml",
    service: "python",
    description: "Python Version from PEP 621 TOML",
    params: &[Param {
        name: "url",
        required: true,
        example: "https://raw.githubusercontent.com/numpy/numpy/main/pyproject.toml",
    }],
    numeric: false,
    resolve: resolve_version_from_toml,
}];
