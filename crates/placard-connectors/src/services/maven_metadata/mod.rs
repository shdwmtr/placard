use super::meta::{Param, PresetMeta};
mod maven_metadata;

pub(crate) use maven_metadata::resolve_maven_metadata;

pub(crate) const PRESETS: &[PresetMeta] = &[PresetMeta {
    preset: "maven-metadata",
    service: "maven_metadata",
    description: "Maven metadata URL",
    params: &[Param {
        name: "metadata_url",
        required: true,
        example: "https://repo1.maven.org/maven2/com/google/guava/guava/maven-metadata.xml",
    }],
    numeric: false,
    resolve: resolve_maven_metadata,
}];
