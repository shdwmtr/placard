use super::meta::{Param, PresetMeta};
mod gerrit;

pub(crate) use gerrit::resolve_gerrit;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "gerrit",
    service: "gerrit",
    description: "Gerrit change status",
    params: &[
        Param {
            name: "change-id",
            required: true,
            example: "1011478",
        },
        Param {
            name: "base-url",
            required: true,
            example: "https://android-review.googlesource.com",
        },
    ],
    numeric: false,
    resolve: resolve_gerrit,
}];
