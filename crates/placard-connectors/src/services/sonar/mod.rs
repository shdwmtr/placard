use super::meta::{Param, PresetMeta};
mod coverage;
mod documented_api_density;
mod fortify_rating;
mod generic;
mod quality_gate;
mod tech_debt;
mod tests;
mod violations;

pub(crate) use coverage::resolve_coverage;
pub(crate) use documented_api_density::resolve_documented_api_density;
pub(crate) use fortify_rating::resolve_fortify_rating;
pub(crate) use generic::resolve_generic;
pub(crate) use quality_gate::resolve_quality_gate;
pub(crate) use tech_debt::resolve_tech_debt;
pub(crate) use tests::resolve_tests;
pub(crate) use violations::resolve_violations;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "sonar-coverage",
        service: "sonar",
        description: "Sonar Coverage",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "gitify-app_gitify",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_coverage,
    },
    PresetMeta {
        preset: "sonar-documented-api-density",
        service: "sonar",
        description: "Sonar Documented API Density",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "brave_brave-core",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_documented_api_density,
    },
    PresetMeta {
        preset: "sonar-fortify-rating",
        service: "sonar",
        description: "Sonar Fortify Security Rating",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "michelin_kstreamplify",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_fortify_rating,
    },
    PresetMeta {
        preset: "sonar-generic",
        service: "sonar",
        description: "",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "",
            },
            Param {
                name: "metric",
                required: true,
                example: "",
            },
            Param {
                name: "branch",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_generic,
    },
    PresetMeta {
        preset: "sonar-quality-gate",
        service: "sonar",
        description: "Sonar Quality Gate",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "michelin_kstreamplify",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_quality_gate,
    },
    PresetMeta {
        preset: "sonar-tech-debt",
        service: "sonar",
        description: "Sonar Tech Debt",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "brave_brave-core",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_tech_debt,
    },
    PresetMeta {
        preset: "sonar-tests",
        service: "sonar",
        description: "Sonar Tests",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "michelin_kstreamplify",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_tests,
    },
    PresetMeta {
        preset: "sonar-violations",
        service: "sonar",
        description: "Sonar Violations",
        params: &[
            Param {
                name: "server",
                required: true,
                example: "",
            },
            Param {
                name: "component",
                required: true,
                example: "brave_brave-core",
            },
            Param {
                name: "metric",
                required: false,
                example: "",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_violations,
    },
];
