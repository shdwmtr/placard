use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod collection_downloads;
mod collection_version;
mod role;

pub(crate) use collection_downloads::resolve_collection_downloads;
pub(crate) use collection_version::resolve_collection_version;
pub(crate) use role::resolve_role;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "ansible-collection-downloads",
        service: "ansible",
        description: "Ansible Collection Downloads",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "community",
            },
            Param {
                name: "name",
                required: true,
                example: "general",
            },
        ],
        numeric: true,
        resolve: resolve_collection_downloads,
    },
    PresetMeta {
        preset: "ansible-collection-version",
        service: "ansible",
        description: "Ansible Collection Version",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "community",
            },
            Param {
                name: "name",
                required: true,
                example: "general",
            },
        ],
        numeric: false,
        resolve: resolve_collection_version,
    },
    PresetMeta {
        preset: "ansible-role",
        service: "ansible",
        description: "Ansible Role",
        params: &[
            Param {
                name: "namespace",
                required: true,
                example: "openwisp",
            },
            Param {
                name: "name",
                required: true,
                example: "openwisp2",
            },
        ],
        numeric: true,
        resolve: resolve_role,
    },
];
