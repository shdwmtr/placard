use super::meta::{Param, PresetMeta};
mod rating;
mod rating_count;
mod release_date;
mod status;
mod version;

pub(crate) use rating::resolve_rating;
pub(crate) use rating_count::resolve_rating_count;
pub(crate) use release_date::resolve_release_date;
pub(crate) use status::resolve_status;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "vaadin-directory-rating",
        service: "vaadin_directory",
        description: "Vaadin Directory Rating",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "vaadinvaadin-grid",
        }],
        numeric: false,
        resolve: resolve_rating,
    },
    PresetMeta {
        preset: "vaadin-directory-rating-count",
        service: "vaadin_directory",
        description: "Vaadin Directory Rating Count",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "vaadinvaadin-grid",
        }],
        numeric: true,
        resolve: resolve_rating_count,
    },
    PresetMeta {
        preset: "vaadin-directory-release-date",
        service: "vaadin_directory",
        description: "Vaadin Directory Release Date",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "vaadinvaadin-grid",
        }],
        numeric: false,
        resolve: resolve_release_date,
    },
    PresetMeta {
        preset: "vaadin-directory-status",
        service: "vaadin_directory",
        description: "Vaadin Directory Status",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "vaadinvaadin-grid",
        }],
        numeric: false,
        resolve: resolve_status,
    },
    PresetMeta {
        preset: "vaadin-directory-version",
        service: "vaadin_directory",
        description: "Vaadin Directory Version",
        params: &[Param {
            name: "package-name",
            required: true,
            example: "vaadinvaadin-grid",
        }],
        numeric: false,
        resolve: resolve_version,
    },
];
