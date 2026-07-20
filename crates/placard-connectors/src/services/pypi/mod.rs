use super::meta::{Param, PresetMeta};
use super::validate_path_param;

mod django_versions;
mod downloads;
mod format;
mod framework_versions;
mod implementation;
mod license;
mod python_versions;
mod status;
mod types;
mod version;
mod wheel;

pub(crate) use django_versions::resolve_django_versions;
pub(crate) use downloads::resolve_downloads;
pub(crate) use format::resolve_format;
pub(crate) use framework_versions::resolve_framework_versions;
pub(crate) use implementation::resolve_implementation;
pub(crate) use license::resolve_license;
pub(crate) use python_versions::resolve_python_versions;
pub(crate) use status::resolve_status;
pub(crate) use types::resolve_types;
pub(crate) use version::resolve_version;
pub(crate) use wheel::resolve_wheel;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "pypi-django-versions",
        service: "pypi",
        description: "",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_django_versions,
    },
    PresetMeta {
        preset: "pypi-downloads",
        service: "pypi",
        description: "PyPI Downloads",
        params: &[
            Param {
                name: "package",
                required: true,
                example: "Django",
            },
            Param {
                name: "period",
                required: true,
                example: "https://pypi.org",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "pypi-format",
        service: "pypi",
        description: "PyPI Format",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_format,
    },
    PresetMeta {
        preset: "pypi-framework-versions",
        service: "pypi",
        description: "PyPI Versions from Framework Classifiers",
        params: &[
            Param {
                name: "framework",
                required: true,
                example: "",
            },
            Param {
                name: "package",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_framework_versions,
    },
    PresetMeta {
        preset: "pypi-implementation",
        service: "pypi",
        description: "PyPI Implementation",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_implementation,
    },
    PresetMeta {
        preset: "pypi-license",
        service: "pypi",
        description: "PyPI License",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "pypi-python-versions",
        service: "pypi",
        description: "PyPI Python Version",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_python_versions,
    },
    PresetMeta {
        preset: "pypi-status",
        service: "pypi",
        description: "PyPI Status",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_status,
    },
    PresetMeta {
        preset: "pypi-types",
        service: "pypi",
        description: "PyPI Types",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_types,
    },
    PresetMeta {
        preset: "pypi-version",
        service: "pypi",
        description: "PyPI Version",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_version,
    },
    PresetMeta {
        preset: "pypi-wheel",
        service: "pypi",
        description: "PyPI Wheel",
        params: &[Param {
            name: "package",
            required: true,
            example: "",
        }],
        numeric: false,
        resolve: resolve_wheel,
    },
];
