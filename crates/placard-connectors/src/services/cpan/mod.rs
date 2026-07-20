use super::meta::{Param, PresetMeta};
mod license;
mod version;

pub(crate) use license::resolve_license;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "cpan-license",
        service: "cpan",
        description: "CPAN License",
        params: &[Param {
            name: "package",
            required: true,
            example: "Config-Augeas",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "cpan-version",
        service: "cpan",
        description: "CPAN Version",
        params: &[Param {
            name: "package",
            required: true,
            example: "Config-Augeas",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
