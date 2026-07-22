use super::meta::PresetMeta;

mod placards_rendered;

pub use placards_rendered::PLACARDS_RENDERED_URL;
pub(crate) use placards_rendered::resolve_placards_rendered;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "placards-rendered",
    service: "placard",
    description: "Total badges rendered by placard.cc",
    params: &[],
    numeric: true,
    resolve: resolve_placards_rendered,
}];
