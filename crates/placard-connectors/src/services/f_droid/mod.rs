use super::meta::{Param, PresetMeta};
mod f_droid;

pub(crate) use f_droid::resolve_f_droid;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "f-droid",
    service: "f_droid",
    description: "F-Droid Version",
    params: &[
        Param {
            name: "app-id",
            required: true,
            example: "org.dystopia.email",
        },
        Param {
            name: "base-url",
            required: false,
            example: "https://apt.izzysoft.de/fdroid",
        },
    ],
    numeric: false,
    resolve: resolve_f_droid,
}];
