use super::meta::{Param, PresetMeta};
use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

mod actions_workflow_status;
mod all_contributors;
mod check_runs;
mod check_suites;
mod checks_status;
mod code_size;
mod commit_status;
mod commits_difference;
mod commits_since;
mod created_at;
mod downloads;
mod followers;
mod forks;
mod go_mod;
mod issue_detail;
mod labels;
mod language_count;
mod last_commit;
mod lerna_json;
mod license;
mod manifest;
mod milestone;
mod milestone_detail;
mod package_json;
mod pipenv;
mod pull_request_check_state;
mod r_package;
mod release;
mod release_date;
mod repo_size;
mod size;
mod tag;
mod top_language;
mod total_star;
mod watchers;

pub(crate) use actions_workflow_status::resolve_actions_workflow_status;
pub(crate) use all_contributors::resolve_all_contributors;
pub(crate) use check_runs::resolve_check_runs;
pub(crate) use check_suites::resolve_check_suites;
pub(crate) use checks_status::resolve_checks_status;
pub(crate) use code_size::resolve_code_size;
pub(crate) use commit_status::resolve_commit_status;
pub(crate) use commits_difference::resolve_commits_difference;
pub(crate) use commits_since::resolve_commits_since;
pub(crate) use created_at::resolve_created_at;
pub(crate) use downloads::resolve_downloads;
pub(crate) use followers::resolve_followers;
pub(crate) use forks::resolve_forks;
pub(crate) use go_mod::resolve_go_mod;
pub(crate) use issue_detail::resolve_issue_detail;
pub(crate) use labels::resolve_labels;
pub(crate) use language_count::resolve_language_count;
pub(crate) use last_commit::resolve_last_commit;
pub(crate) use lerna_json::resolve_lerna_json;
pub(crate) use license::resolve_license;
pub(crate) use manifest::resolve_manifest;
pub(crate) use milestone::resolve_milestone;
pub(crate) use milestone_detail::resolve_milestone_detail;
pub(crate) use package_json::resolve_package_json;
pub(crate) use pipenv::resolve_pipenv;
pub(crate) use pull_request_check_state::resolve_pull_request_check_state;
pub(crate) use r_package::resolve_r_package;
pub(crate) use release::resolve_release;
pub(crate) use release_date::resolve_release_date;
pub(crate) use repo_size::resolve_repo_size;
pub(crate) use size::resolve_size;
pub(crate) use tag::resolve_tag;
pub(crate) use top_language::resolve_top_language;
pub(crate) use total_star::resolve_total_star;
pub(crate) use watchers::resolve_watchers;

pub(crate) const PRESETS: &[PresetMeta] = &[
    PresetMeta {
        preset: "github-actions-workflow-status",
        service: "github",
        description: "GitHub Actions Workflow Status",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "actions",
            },
            Param {
                name: "repo",
                required: true,
                example: "toolkit",
            },
            Param {
                name: "workflow",
                required: true,
                example: "unit-tests.yml",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_actions_workflow_status,
    },
    PresetMeta {
        preset: "github-all-contributors",
        service: "github",
        description: "GitHub contributors from allcontributors.org (with branch)",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "all-contributors",
            },
            Param {
                name: "repo",
                required: true,
                example: "all-contributors",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: true,
        resolve: resolve_all_contributors,
    },
    PresetMeta {
        preset: "github-check-runs",
        service: "github",
        description: "GitHub branch check runs",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "ref",
                required: true,
                example: "",
            },
            Param {
                name: "name",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_check_runs,
    },
    PresetMeta {
        preset: "github-check-suites",
        service: "github",
        description: "GitHub branch check suites",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "ref",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_check_suites,
    },
    PresetMeta {
        preset: "github-checks-status",
        service: "github",
        description: "GitHub branch status",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "ref",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_checks_status,
    },
    PresetMeta {
        preset: "github-code-size",
        service: "github",
        description: "GitHub code size in bytes",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
        ],
        numeric: true,
        resolve: resolve_code_size,
    },
    PresetMeta {
        preset: "github-commit-status",
        service: "github",
        description: "GitHub commit merge status",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "branch",
                required: true,
                example: "master",
            },
            Param {
                name: "commit",
                required: true,
                example: "5d4ab86b1b5ddfb3c4a70a70bd19932c52603b8c",
            },
        ],
        numeric: false,
        resolve: resolve_commit_status,
    },
    PresetMeta {
        preset: "github-commits-difference",
        service: "github",
        description: "GitHub commits difference between two branches/tags/commits",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "microsoft",
            },
            Param {
                name: "repo",
                required: true,
                example: "vscode",
            },
            Param {
                name: "base",
                required: true,
                example: "1.60.0",
            },
            Param {
                name: "head",
                required: true,
                example: "82f2db7",
            },
        ],
        numeric: true,
        resolve: resolve_commits_difference,
    },
    PresetMeta {
        preset: "github-commits-since",
        service: "github",
        description: "GitHub commits since tagged version",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "SubtitleEdit",
            },
            Param {
                name: "repo",
                required: true,
                example: "subtitleedit",
            },
            Param {
                name: "version",
                required: true,
                example: "3.4.7",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: true,
        resolve: resolve_commits_since,
    },
    PresetMeta {
        preset: "github-created-at",
        service: "github",
        description: "GitHub Created At",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "mashape",
            },
            Param {
                name: "repo",
                required: true,
                example: "apistatus",
            },
        ],
        numeric: false,
        resolve: resolve_created_at,
    },
    PresetMeta {
        preset: "github-downloads",
        service: "github",
        description: "GitHub Downloads (all assets, all releases)",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "atom",
            },
            Param {
                name: "repo",
                required: true,
                example: "atom",
            },
            Param {
                name: "tag",
                required: false,
                example: "v0.190.0",
            },
            Param {
                name: "variant",
                required: false,
                example: "downloads-pre",
            },
            Param {
                name: "sort",
                required: false,
                example: "date",
            },
            Param {
                name: "asset_name",
                required: false,
                example: "total",
            },
        ],
        numeric: true,
        resolve: resolve_downloads,
    },
    PresetMeta {
        preset: "github-followers",
        service: "github",
        description: "GitHub followers",
        params: &[Param {
            name: "user",
            required: true,
            example: "espadrine",
        }],
        numeric: true,
        resolve: resolve_followers,
    },
    PresetMeta {
        preset: "github-forks",
        service: "github",
        description: "GitHub forks",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
        ],
        numeric: true,
        resolve: resolve_forks,
    },
    PresetMeta {
        preset: "github-go-mod",
        service: "github",
        description: "GitHub go.mod Go version",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "gohugoio",
            },
            Param {
                name: "repo",
                required: true,
                example: "hugo",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_go_mod,
    },
    PresetMeta {
        preset: "github-issue-detail",
        service: "github",
        description: "GitHub issue/pull request detail",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "number",
                required: true,
                example: "979",
            },
            Param {
                name: "property",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_issue_detail,
    },
    PresetMeta {
        preset: "github-labels",
        service: "github",
        description: "GitHub labels",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "atom",
            },
            Param {
                name: "repo",
                required: true,
                example: "atom",
            },
            Param {
                name: "name",
                required: true,
                example: "help-wanted",
            },
        ],
        numeric: false,
        resolve: resolve_labels,
    },
    PresetMeta {
        preset: "github-language-count",
        service: "github",
        description: "GitHub language count",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
        ],
        numeric: true,
        resolve: resolve_language_count,
    },
    PresetMeta {
        preset: "github-last-commit",
        service: "github",
        description: "GitHub last commit",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "google",
            },
            Param {
                name: "repo",
                required: true,
                example: "skia",
            },
            Param {
                name: "branch",
                required: false,
                example: "infra/config",
            },
            Param {
                name: "path",
                required: false,
                example: "",
            },
            Param {
                name: "display-timestamp",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_last_commit,
    },
    PresetMeta {
        preset: "github-lerna-json",
        service: "github",
        description: "GitHub lerna version",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "babel",
            },
            Param {
                name: "repo",
                required: true,
                example: "babel",
            },
            Param {
                name: "branch",
                required: false,
                example: "colors",
            },
        ],
        numeric: false,
        resolve: resolve_lerna_json,
    },
    PresetMeta {
        preset: "github-license",
        service: "github",
        description: "GitHub License",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "mashape",
            },
            Param {
                name: "repo",
                required: true,
                example: "apistatus",
            },
        ],
        numeric: false,
        resolve: resolve_license,
    },
    PresetMeta {
        preset: "github-manifest",
        service: "github",
        description: "GitHub manifest version",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "sindresorhus",
            },
            Param {
                name: "repo",
                required: true,
                example: "show-all-github-issues",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
            Param {
                name: "filename",
                required: false,
                example: "extension/manifest.json",
            },
            Param {
                name: "key",
                required: false,
                example: "permissions",
            },
        ],
        numeric: false,
        resolve: resolve_manifest,
    },
    PresetMeta {
        preset: "github-milestone",
        service: "github",
        description: "GitHub number of milestones",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "variant",
                required: true,
                example: "",
            },
        ],
        numeric: true,
        resolve: resolve_milestone,
    },
    PresetMeta {
        preset: "github-milestone-detail",
        service: "github",
        description: "GitHub milestone details",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "number",
                required: true,
                example: "1",
            },
            Param {
                name: "variant",
                required: true,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_milestone_detail,
    },
    PresetMeta {
        preset: "github-package-json",
        service: "github",
        description: "GitHub package.json version",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "key",
                required: true,
                example: "badge-maker/package.json",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: false,
        resolve: resolve_package_json,
    },
    PresetMeta {
        preset: "github-pipenv",
        service: "github",
        description: "GitHub Pipenv locked Python version",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "metabolize",
            },
            Param {
                name: "repo",
                required: true,
                example: "rq-dashboard-on-heroku",
            },
            Param {
                name: "branch",
                required: false,
                example: "main",
            },
        ],
        numeric: false,
        resolve: resolve_pipenv,
    },
    PresetMeta {
        preset: "github-pull-request-check-state",
        service: "github",
        description: "GitHub pull request status",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
            Param {
                name: "number",
                required: true,
                example: "1110",
            },
        ],
        numeric: false,
        resolve: resolve_pull_request_check_state,
    },
    PresetMeta {
        preset: "github-r-package",
        service: "github",
        description: "GitHub R package version",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "mixOmicsTeam",
            },
            Param {
                name: "repo",
                required: true,
                example: "mixOmics",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
            Param {
                name: "filename",
                required: false,
                example: "subdirectory/DESCRIPTION",
            },
        ],
        numeric: false,
        resolve: resolve_r_package,
    },
    PresetMeta {
        preset: "github-release",
        service: "github",
        description: "GitHub Release",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "expressjs",
            },
            Param {
                name: "repo",
                required: true,
                example: "express",
            },
            Param {
                name: "display_name",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_release,
    },
    PresetMeta {
        preset: "github-release-date",
        service: "github",
        description: "GitHub Release Date",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "SubtitleEdit",
            },
            Param {
                name: "repo",
                required: true,
                example: "subtitleedit",
            },
            Param {
                name: "variant",
                required: false,
                example: "",
            },
            Param {
                name: "display_date",
                required: false,
                example: "",
            },
        ],
        numeric: false,
        resolve: resolve_release_date,
    },
    PresetMeta {
        preset: "github-repo-size",
        service: "github",
        description: "GitHub repo size",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "atom",
            },
            Param {
                name: "repo",
                required: true,
                example: "atom",
            },
        ],
        numeric: true,
        resolve: resolve_repo_size,
    },
    PresetMeta {
        preset: "github-size",
        service: "github",
        description: "GitHub file size in bytes",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "webcaetano",
            },
            Param {
                name: "repo",
                required: true,
                example: "craft",
            },
            Param {
                name: "path",
                required: true,
                example: "build/phaser-craft.min.js",
            },
            Param {
                name: "branch",
                required: false,
                example: "master",
            },
        ],
        numeric: true,
        resolve: resolve_size,
    },
    PresetMeta {
        preset: "github-stars",
        service: "github",
        description: "GitHub Repo stars",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
        ],
        numeric: true,
        resolve: resolve_stars,
    },
    PresetMeta {
        preset: "github-tag",
        service: "github",
        description: "GitHub Tag",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "expressjs",
            },
            Param {
                name: "repo",
                required: true,
                example: "express",
            },
        ],
        numeric: false,
        resolve: resolve_tag,
    },
    PresetMeta {
        preset: "github-top-language",
        service: "github",
        description: "GitHub top language",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
        ],
        numeric: false,
        resolve: resolve_top_language,
    },
    PresetMeta {
        preset: "github-total-star",
        service: "github",
        description: "GitHub User's stars",
        params: &[Param {
            name: "user",
            required: true,
            example: "chris48s",
        }],
        numeric: true,
        resolve: resolve_total_star,
    },
    PresetMeta {
        preset: "github-watchers",
        service: "github",
        description: "GitHub watchers",
        params: &[
            Param {
                name: "owner",
                required: true,
                example: "badges",
            },
            Param {
                name: "repo",
                required: true,
                example: "shields",
            },
        ],
        numeric: true,
        resolve: resolve_watchers,
    },
];

pub(crate) fn resolve_stars(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-stars requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-stars requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let count = value
        .get("stargazers_count")
        .ok_or("github response missing stargazers_count")?;
    count
        .as_text()
        .ok_or_else(|| "stargazers_count was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://api.github.com/repos/shdwmtr/placard");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(owner: &str, repo: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), owner.to_string()),
            ("repo".to_string(), repo.to_string()),
        ])
    }

    #[test]
    fn extracts_stargazers_count_from_a_github_shaped_response() {
        let fetcher = FakeFetcher(r#"{"id": 1, "name": "placard", "stargazers_count": 12483}"#);
        let value = resolve_stars(&params("shdwmtr", "placard"), &fetcher).unwrap();
        assert_eq!(value, "12483");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_stars(&HashMap::new(), &Unused).is_err());
        assert!(resolve_stars(&params("shdwmtr", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_stars(&params("../etc", "placard"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"id": 1}"#);
        assert!(resolve_stars(&params("shdwmtr", "placard"), &fetcher).is_err());
    }
}
