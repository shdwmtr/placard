use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn validate_job_url(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'job-url' parameter must not be empty".to_string());
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err("'job-url' parameter must be an absolute http(s) URL".to_string());
    }
    if value.chars().any(|c| c.is_whitespace()) {
        return Err("'job-url' parameter must not contain whitespace".to_string());
    }
    Ok(value.trim_end_matches('/'))
}

fn status_for_color(color: &str) -> Option<&'static str> {
    Some(match color {
        "red" => "failing",
        "red_anime" => "building",
        "yellow" => "unstable",
        "yellow_anime" => "building",
        "blue" => "passing",
        "blue_anime" => "building",
        "green" => "passing",
        "green_anime" => "building",
        "grey" => "not built",
        "grey_anime" => "building",
        "disabled" => "not built",
        "disabled_anime" => "building",
        "aborted" => "not built",
        "aborted_anime" => "building",
        "notbuilt" => "not built",
        "notbuilt_anime" => "building",
        _ => return None,
    })
}

pub(crate) fn resolve_build(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let job_url = params
        .get("job-url")
        .ok_or("jenkins-build requires a data-job-url attribute")?;
    let job_url = validate_job_url(job_url)?;

    let url = format!("{job_url}/api/json?tree=color");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "jenkins response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let color = value.get("color").ok_or("jenkins response missing color")?;
    let color = color
        .as_text()
        .ok_or_else(|| "color was not a plain value".to_string())?;
    status_for_color(&color)
        .map(str::to_string)
        .ok_or_else(|| format!("unrecognized jenkins build color '{color}'"))
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

    fn params(job_url: &str) -> HashMap<String, String> {
        HashMap::from([("job-url".to_string(), job_url.to_string())])
    }

    #[test]
    fn maps_blue_to_passing() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{"color": "blue"}"#,
        };
        let value =
            resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn maps_red_to_failing() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{"color": "red"}"#,
        };
        let value =
            resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "failing");
    }

    #[test]
    fn maps_yellow_to_unstable() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{"color": "yellow"}"#,
        };
        let value =
            resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "unstable");
    }

    #[test]
    fn maps_anime_colors_to_building() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{"color": "red_anime"}"#,
        };
        let value =
            resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "building");
    }

    #[test]
    fn maps_grey_to_not_built() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{"color": "grey"}"#,
        };
        let value =
            resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).unwrap();
        assert_eq!(value, "not built");
    }

    #[test]
    fn trims_a_trailing_slash_from_job_url() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{"color": "blue"}"#,
        };
        let value =
            resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit/"), &fetcher).unwrap();
        assert_eq!(value, "passing");
    }

    #[test]
    fn requires_job_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid job-url")
            }
        }
        assert!(resolve_build(&HashMap::new(), &Unused).is_err());
        assert!(resolve_build(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_a_non_http_job_url() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid job-url")
            }
        }
        assert!(resolve_build(&params("ftp://example.com/job/x"), &Unused).is_err());
        assert!(resolve_build(&params("not a url"), &Unused).is_err());
    }

    #[test]
    fn errors_on_an_unrecognized_color() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{"color": "purple"}"#,
        };
        assert!(resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).is_err());
    }

    #[test]
    fn errors_when_color_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://ci.eclipse.org/jgit/job/jgit/api/json?tree=color",
            body: r#"{}"#,
        };
        assert!(resolve_build(&params("https://ci.eclipse.org/jgit/job/jgit"), &fetcher).is_err());
    }
}
