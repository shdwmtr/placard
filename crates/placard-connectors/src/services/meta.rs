use crate::Fetcher;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    pub required: bool,
    pub example: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct PresetMeta {
    pub preset: &'static str,
    pub service: &'static str,
    pub description: &'static str,
    pub params: &'static [Param],
    /// Whether this preset's resolved value is a plain number (safe to
    /// reformat with `data-number-format`) as opposed to text -- a version
    /// string, a license name, a status word, a date, or anything else that
    /// isn't a bare number. Not inferred at runtime: `data-number-format`
    /// still works on any preset regardless of this flag (it's a no-op if
    /// the resolved value doesn't parse as a number), but this is what lets
    /// the docs only *advertise* the feature where it actually applies.
    pub numeric: bool,
    pub resolve: fn(&HashMap<String, String>, &dyn Fetcher) -> Result<String, String>,
}

pub fn param_options(preset: &str, param: &str) -> &'static [&'static str] {
    match (preset, param) {
        ("bundlephobia", "format") => &["min", "minzip"],
        ("cii-best-practices", "metric") => &["level", "percentage", "summary"],
        ("circleci", "vcs-type") => &["github", "gh", "bitbucket", "bb"],
        ("codecov", "vcs-name") => &["github", "gh", "bitbucket", "bb", "gl", "gitlab"],
        ("codefactor-grade", "vcs-type") => &["github", "bitbucket"],
        ("coderabbit-pull-request", "provider") => &["github", "bitbucket", "gitlab"],
        ("coveralls", "vcs-type") => &["github", "bitbucket", "gitlab"],
        ("crates-downloads", "variant") => &["d", "dv", "dr"],
        ("depfu", "vcs-type") => &["github", "gitlab"],
        ("deps-rs-repo", "site") => &["github", "gitlab", "bitbucket", "sourcehut", "codeberg"],
        ("discourse", "variant") => &["topics", "users", "posts", "likes", "status"],
        ("dub-download", "interval") => &["dd", "dw", "dm", "dt"],
        ("eclipse-marketplace-downloads", "interval") => &["dm", "dt"],
        ("feedz", "variant") => &["v", "vpre"],
        ("gem-downloads", "variant") => &["dt", "dtv", "dv"],
        ("gem-rank", "period") => &["rt", "rd"],
        ("gitea-last-commit", "display-timestamp") => &["author", "committer"],
        ("gitea-release", "display-name") => &["tag", "release"],
        ("github-downloads", "variant") => &["downloads", "downloads-pre"],
        ("github-downloads", "sort") => &["date"],
        ("github-downloads", "asset_name") => &["total"],
        ("github-issue-detail", "property") => &[
            "state",
            "title",
            "author",
            "comments",
            "milestone",
            "age",
            "last-update",
            "label",
        ],
        ("github-last-commit", "display-timestamp") => &["author", "committer"],
        ("github-milestone", "variant") => &["open", "closed", "all"],
        ("github-milestone-detail", "variant") => &[
            "issues-open",
            "issues-closed",
            "issues-total",
            "progress",
            "progress-percent",
        ],
        ("github-release", "display_name") => &["tag", "release"],
        ("github-release-date", "variant") => &["release-date", "release-date-pre"],
        ("github-release-date", "display_date") => &["created_at", "published_at"],
        ("gitlab-issues", "variant") => {
            &["all", "all-raw", "open", "open-raw", "closed", "closed-raw"]
        }
        ("gitlab-release", "sort") => &["date"],
        ("gitlab-release", "display-name") => &["tag", "release"],
        ("gitlab-release", "date-order-by") => &["created_at", "released_at"],
        ("gitlab-tag", "sort") => &["date"],
        ("greasyfork-downloads", "variant") => &["dt", "dd"],
        ("hexpm-downloads", "interval") => &["dd", "dw", "dt"],
        ("homebrew-cask-downloads", "interval") => &["dm", "dq", "dy"],
        ("homebrew-formula-downloads", "interval") => &["dm", "dq", "dy"],
        ("jetbrains-rating", "format") => &["rating", "stars"],
        ("jsdelivr-hits-github", "period") => &["hd", "hw", "hm", "hy"],
        ("jsdelivr-hits-npm", "period") => &["hd", "hw", "hm", "hy"],
        ("npm-downloads", "interval") => &["dw", "dm", "dy", "d18m"],
        ("npm-stat-downloads", "interval") => &["dw", "dm", "dy"],
        ("npms-io-score", "type") => &[
            "final-score",
            "maintenance-score",
            "popularity-score",
            "quality-score",
        ],
        ("nuget-version", "variant") => &["v", "vpre"],
        ("nycrc", "preferred-threshold") => &["branches", "lines", "functions"],
        ("open-vsx-rating", "format") => &["rating", "stars"],
        ("packagecontrol", "interval") => &["dd", "dw", "dm", "dt"],
        ("packagist-downloads", "interval") => &["dd", "dm", "dt"],
        ("polymart-rating", "format") => &["rating", "stars"],
        ("pypi-downloads", "period") => &["dd", "dw", "dm"],
        ("pypi-framework-versions", "framework") => &[
            "aws-cdk",
            "django",
            "django-cms",
            "jupyterlab",
            "odoo",
            "plone",
            "wagtail",
            "zope",
        ],
        ("sonar-violations", "metric") => &[
            "violations",
            "blocker_violations",
            "critical_violations",
            "major_violations",
            "minor_violations",
            "info_violations",
        ],
        ("sourceforge-downloads", "interval") => &["dd", "dw", "dm", "dt"],
        ("spiget-rating", "format") => &["rating", "stars"],
        ("teamcity-build", "verbosity") => &["s", "e"],
        ("terraform-module-downloads", "interval") => &["dw", "dm", "dy", "dt"],
        ("terraform-provider-downloads", "interval") => &["dw", "dm", "dy", "dt"],
        ("testspace-test-count", "metric") => &[
            "total", "passed", "failed", "skipped", "errored", "untested",
        ],
        ("uptimeobserver-ratio", "period") => &["1", "7"],
        ("voxelshop-rating", "format") => &["rating", "stars"],
        ("w3c-validation", "parser") => &["default", "html", "xml", "xmldtd"],
        ("weblate-entities", "type") => &["components", "projects", "users", "languages"],
        ("weblate-user-statistic", "statistic") => &[
            "translations",
            "suggestions",
            "uploads",
            "comments",
            "languages",
        ],
        ("whatpulse", "metric") => &["keys", "clicks", "uptime", "download", "upload"],
        ("whatpulse", "user-type") => &["user", "team"],
        ("wordpress-downloads", "type") => &["plugin", "theme"],
        ("wordpress-downloads", "interval") => &["dt", "dd", "dw", "dm", "dy"],
        ("wordpress-last-update", "type") => &["plugin", "theme"],
        ("wordpress-platform", "variant") => &["requires", "requires-php", "tested"],
        ("wordpress-platform", "type") => &["plugin", "theme"],
        ("wordpress-rating", "type") => &["plugin", "theme"],
        ("wordpress-version", "type") => &["plugin", "theme"],
        _ => &[],
    }
}
