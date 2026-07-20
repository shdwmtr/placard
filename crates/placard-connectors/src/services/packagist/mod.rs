use super::meta::{Param, PresetMeta};
mod dependency_version;
mod downloads;
mod license;
mod php_version;
mod stars;
mod version;

pub(crate) use dependency_version::resolve_dependency_version;
pub(crate) use downloads::resolve_downloads;
pub(crate) use license::resolve_license;
pub(crate) use php_version::resolve_php_version;
pub(crate) use stars::resolve_stars;
pub(crate) use version::resolve_version;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "packagist-dependency-version",
        service: "packagist",
        description: "Packagist Dependency Version",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "guzzlehttp",
            },
            Param {
                name: "repo",
                required: true,
                example: "guzzle",
            },
            Param {
                name: "dependency",
                required: true,
                example: "php",
            },
            Param {
                name: "version",
                required: false,
                example: "v2.8.0",
            },
            Param {
                name: "server",
                required: false,
                example: "https://packagist.org",
            },
        ],
        numeric: false,
        resolve: resolve_dependency_version,
    },
    PresetMeta {
        preset: "packagist-downloads",
        service: "packagist",
        description: "Packagist Downloads",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "guzzlehttp",
            },
            Param {
                name: "repo",
                required: true,
                example: "guzzle",
            },
            Param {
                name: "interval",
                required: true,
                example: "",
            },
            Param {
                name: "server",
                required: false,
                example: "https://packagist.org",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "packagist-license",
        service: "packagist",
        description: "Packagist License",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "guzzlehttp",
            },
            Param {
                name: "repo",
                required: true,
                example: "guzzle",
            },
            Param {
                name: "server",
                required: false,
                example: "https://packagist.org",
            },
        ],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "packagist-php-version",
        service: "packagist",
        description: "",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "",
            },
            Param {
                name: "repo",
                required: true,
                example: "",
            },
            Param {
                name: "version",
                required: false,
                example: "",
            },
            Param {
                name: "server",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_php_version,
    },
    PresetMeta {
        preset: "packagist-stars",
        service: "packagist",
        description: "Packagist Stars",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "guzzlehttp",
            },
            Param {
                name: "repo",
                required: true,
                example: "guzzle",
            },
            Param {
                name: "server",
                required: false,
                example: "https://packagist.org",
            },
        ],
        numeric: true,
        resolve: resolve_stars,
    },
    PresetMeta {
        preset: "packagist-version",
        service: "packagist",
        description: "Packagist Version",
        params: &[
            Param {
                name: "user",
                required: true,
                example: "symfony",
            },
            Param {
                name: "repo",
                required: true,
                example: "symfony",
            },
            Param {
                name: "server",
                required: false,
                example: "https://packagist.org",
            },
        ],
        numeric: false,
        resolve: resolve_version,
    },
];
