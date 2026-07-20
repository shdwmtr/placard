use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod automated;
mod pulls;
mod size;
mod stars;
mod version;

pub(crate) use automated::resolve_automated;
pub(crate) use pulls::resolve_pulls;
pub(crate) use size::resolve_size;
pub(crate) use stars::resolve_stars;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "docker-automated",
        service: "docker",
        description: "Docker Automated build",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "jrottenberg",
            },
            Param {
                name: "repo",
                required: true,
                example: "ffmpeg",
            },
        ],
        numeric: false,
        resolve: resolve_automated,
    },
    PresetMeta {
        preset: "docker-pulls",
        service: "docker",
        description: "Docker Pulls",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "_",
            },
            Param {
                name: "repo",
                required: true,
                example: "ubuntu",
            },
        ],
        numeric: true,
        resolve: resolve_pulls,
    },
    PresetMeta {
        preset: "docker-size",
        service: "docker",
        description: "Docker Image Size",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "fedora",
            },
            Param {
                name: "repo",
                required: true,
                example: "apache",
            },
            Param {
                name: "tag",
                required: true,
                example: "latest",
            },
        ],
        numeric: true,
        resolve: resolve_size,
    },
    PresetMeta {
        preset: "docker-stars",
        service: "docker",
        description: "Docker Stars",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "_",
            },
            Param {
                name: "repo",
                required: true,
                example: "ubuntu",
            },
        ],
        numeric: true,
        resolve: resolve_stars,
    },
    PresetMeta {
        preset: "docker-version",
        service: "docker",
        description: "Docker Image Version",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "_",
            },
            Param {
                name: "repo",
                required: true,
                example: "alpine",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
