use super::meta::{Param, PresetMeta};
mod gradle_plugin_portal;

pub(crate) use gradle_plugin_portal::resolve_gradle_plugin_portal;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "gradle-plugin-portal",
    service: "gradle_plugin_portal",
    description: "Gradle Plugin Portal Version",
    params: &[Param {
        name: "plugin-id",
        required: true,
        example: "com.gradle.plugin-publish",
    }],
    numeric: false,
    resolve: resolve_gradle_plugin_portal,
}];
