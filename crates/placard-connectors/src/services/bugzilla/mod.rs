use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod bugzilla;

pub(crate) use bugzilla::resolve_bugzilla;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "bugzilla",
    service: "bugzilla",
    description: "Bugzilla bug status",
    params: &[
        Param {
            name: "bug-number",
            required: true,
            example: "12345",
        },
        Param {
            name: "base-url",
            required: false,
            example: "https://gcc.gnu.org/bugzilla",
        },
    ],
    numeric: false,
    resolve: resolve_bugzilla,
}];
