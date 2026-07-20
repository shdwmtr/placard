use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod downloads;
mod latest_version;
mod rating;

pub(crate) use downloads::resolve_downloads;
pub(crate) use latest_version::resolve_latest_version;
pub(crate) use rating::resolve_rating;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "voxelshop-downloads",
        service: "voxelshop",
        description: "Voxel Shop Downloads",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "323",
        }],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "voxelshop-latest-version",
        service: "voxelshop",
        description: "Voxel Shop Version",
        params: &[Param {
            name: "resource-id",
            required: true,
            example: "323",
        }],
        numeric: false,
        resolve: resolve_latest_version,
    },
    PresetMeta {
        preset: "voxelshop-rating",
        service: "voxelshop",
        description: "Voxel Shop Rating",
        params: &[
            Param {
                name: "resource-id",
                required: true,
                example: "323",
            },
            Param {
                name: "format",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_rating,
    },
];
