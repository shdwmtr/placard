use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn validate_segmented_param<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("'{name}' parameter must not be empty"));
    }
    for segment in value.split('/') {
        validate_path_param(name, segment)?;
    }
    Ok(value)
}

pub(crate) fn resolve_r_package(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-r-package requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-r-package requires a data-repo attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let branch = match params.get("branch") {
        Some(branch) => validate_path_param("branch", branch)?,
        None => "HEAD",
    };
    let filename = match params.get("filename") {
        Some(filename) => validate_segmented_param("filename", filename)?,
        None => "DESCRIPTION",
    };

    let url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{filename}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Version:") {
            let version = rest.trim();
            if !version.is_empty() {
                return Ok(version.to_string());
            }
        }
    }
    Err(format!("Version missing in {filename}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/mixOmicsTeam/mixOmics/HEAD/DESCRIPTION"
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
    fn extracts_version_from_description_file() {
        let fetcher = FakeFetcher("Package: mixOmics\nVersion: 6.24.0\nDate: 2023-01-01\n");
        let value = resolve_r_package(&params("mixOmicsTeam", "mixOmics"), &fetcher).unwrap();
        assert_eq!(value, "6.24.0");
    }

    #[test]
    fn uses_custom_branch_and_filename_when_given() {
        struct CustomFetcher;
        impl Fetcher for CustomFetcher {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://raw.githubusercontent.com/mixOmicsTeam/mixOmics/master/subdirectory/DESCRIPTION"
                );
                Ok(b"Version: 1.0.0\n".to_vec())
            }
        }
        let mut p = params("mixOmicsTeam", "mixOmics");
        p.insert("branch".to_string(), "master".to_string());
        p.insert(
            "filename".to_string(),
            "subdirectory/DESCRIPTION".to_string(),
        );
        let value = resolve_r_package(&p, &CustomFetcher).unwrap();
        assert_eq!(value, "1.0.0");
    }

    #[test]
    fn requires_owner_and_repo_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_r_package(&HashMap::new(), &Unused).is_err());
        assert!(resolve_r_package(&params("mixOmicsTeam", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_r_package(&params("../etc", "mixOmics"), &Unused).is_err());
    }

    #[test]
    fn errors_when_version_line_is_missing() {
        let fetcher = FakeFetcher("Package: mixOmics\nDate: 2023-01-01\n");
        assert!(resolve_r_package(&params("mixOmicsTeam", "mixOmics"), &fetcher).is_err());
    }
}
