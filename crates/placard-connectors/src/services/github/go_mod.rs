use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

pub(crate) fn resolve_go_mod(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-go-mod requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-go-mod requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };

    let url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/go.mod");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "go.mod response was not valid UTF-8".to_string())?;

    for line in text.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("go ") {
            let version = rest.split(['/', ' ', '\t']).next().unwrap_or("").trim();
            if !version.is_empty() {
                return Ok(version.to_string());
            }
        }
    }
    Err("go version missing in go.mod".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/gohugoio/hugo/HEAD/go.mod"
            );
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
    fn extracts_go_version_from_a_go_mod_file() {
        let fetcher = FakeFetcher(
            "module github.com/gohugoio/hugo\n\ngo 1.21\n\nrequire (\n\tfoo v1.0.0\n)\n",
        );
        let value = resolve_go_mod(&params("gohugoio", "hugo"), &fetcher).unwrap();
        assert_eq!(value, "1.21");
    }

    #[test]
    fn extracts_go_version_with_a_trailing_toolchain_comment() {
        let fetcher = FakeFetcher("module github.com/gohugoio/hugo\n\ngo 1.21.0 // some comment\n");
        let value = resolve_go_mod(&params("gohugoio", "hugo"), &fetcher).unwrap();
        assert_eq!(value, "1.21.0");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_go_mod(&HashMap::new(), &Unused).is_err());
        assert!(resolve_go_mod(&params("gohugoio", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_go_mod(&params("../etc", "hugo"), &Unused).is_err());
    }

    #[test]
    fn errors_when_go_directive_is_missing() {
        let fetcher =
            FakeFetcher("module github.com/gohugoio/hugo\n\nrequire (\n\tfoo v1.0.0\n)\n");
        assert!(resolve_go_mod(&params("gohugoio", "hugo"), &fetcher).is_err());
    }
}
