use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

/// Netlify deploy status badges are SVGs whose fill color encodes the
/// status; there is no JSON API for this, so we scrape the same fixed
/// hex colors shields itself checks for.
fn status_from_svg(svg: &str) -> &'static str {
    if svg.contains("#0F4A21") {
        "passing"
    } else if svg.contains("#800A20") {
        "failing"
    } else if svg.contains("#603408") {
        "building"
    } else if svg.contains("#181A1C") {
        "canceled"
    } else {
        "unknown"
    }
}

pub(crate) fn resolve_netlify(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let project_id = params
        .get("project-id")
        .ok_or("netlify requires a data-project-id attribute")?;
    let project_id = validate_path_param("project-id", project_id)?;

    let url = format!("https://api.netlify.com/api/v1/badges/{project_id}/deploy-status");
    let bytes = fetcher.fetch(&url)?;
    let svg =
        String::from_utf8(bytes).map_err(|_| "netlify response was not valid UTF-8".to_string())?;
    Ok(status_from_svg(&svg).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: &'static str,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(project_id: &str) -> HashMap<String, String> {
        HashMap::from([("project-id".to_string(), project_id.to_string())])
    }

    #[test]
    fn extracts_passing_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.netlify.com/api/v1/badges/e6d5a4e0-dee1-4261-833e-2f47f509c68f/deploy-status",
            body: r##"<svg><path fill="#0F4A21"/></svg>"##,
        };
        let value =
            resolve_netlify(&params("e6d5a4e0-dee1-4261-833e-2f47f509c68f"), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn extracts_failing_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.netlify.com/api/v1/badges/abc/deploy-status",
            body: r##"<svg><path fill="#800A20"/></svg>"##,
        };
        let value = resolve_netlify(&params("abc"), &fetcher).unwrap();
        assert_eq!(value, "failing");
    }

    #[test]
    fn falls_back_to_unknown_status() {
        let fetcher = FakeFetcher {
            expected_url: "https://api.netlify.com/api/v1/badges/abc/deploy-status",
            body: r##"<svg><path fill="#000000"/></svg>"##,
        };
        let value = resolve_netlify(&params("abc"), &fetcher).unwrap();
        assert_eq!(value, "unknown");
    }

    #[test]
    fn requires_project_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid project id")
            }
        }
        assert!(resolve_netlify(&HashMap::new(), &Unused).is_err());
        assert!(resolve_netlify(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid project id")
            }
        }
        assert!(resolve_netlify(&params("../etc/passwd"), &Unused).is_err());
    }
}
