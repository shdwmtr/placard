use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn validate_clojar(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("'clojar' parameter must not be empty".to_string());
    }
    for segment in value.split('/') {
        if segment == "." || segment == ".." {
            return Err("'clojar' parameter contains disallowed characters".to_string());
        }
        validate_path_param("clojar", segment)?;
    }
    Ok(value)
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let clojar = params
        .get("clojar")
        .ok_or("clojars-version requires a data-clojar attribute")?;
    let clojar = validate_clojar(clojar)?;
    let include_prereleases = params
        .get("include_prereleases")
        .is_some_and(|v| v != "false");

    let url = format!("https://clojars.org/api/artifacts/{clojar}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "clojars response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let version = if include_prereleases {
        value
            .get("latest_version")
            .ok_or("clojars response missing latest_version")?
    } else {
        match value.get("latest_release") {
            Some(Value::Null) | None => value
                .get("latest_version")
                .ok_or("clojars response missing latest_version")?,
            Some(release) => release,
        }
    };
    let version = version
        .as_text()
        .ok_or_else(|| "version was not a plain value".to_string())?;
    Ok(format!("[{clojar} \"{version}\"]"))
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

    fn params(clojar: &str) -> HashMap<String, String> {
        HashMap::from([("clojar".to_string(), clojar.to_string())])
    }

    #[test]
    fn extracts_the_latest_release_by_default() {
        let fetcher = FakeFetcher {
            expected_url: "https://clojars.org/api/artifacts/prismic",
            body: r#"{"downloads": 4213, "latest_release": "7.0.0", "latest_version": "7.0.1-SNAPSHOT"}"#,
        };
        let value = resolve_version(&params("prismic"), &fetcher).unwrap();
        assert_eq!(value, "[prismic \"7.0.0\"]");
    }

    #[test]
    fn falls_back_to_latest_version_when_no_release_exists() {
        let fetcher = FakeFetcher {
            expected_url: "https://clojars.org/api/artifacts/prismic",
            body: r#"{"downloads": 4213, "latest_release": null, "latest_version": "7.0.1-SNAPSHOT"}"#,
        };
        let value = resolve_version(&params("prismic"), &fetcher).unwrap();
        assert_eq!(value, "[prismic \"7.0.1-SNAPSHOT\"]");
    }

    #[test]
    fn uses_latest_version_when_include_prereleases_is_set() {
        let fetcher = FakeFetcher {
            expected_url: "https://clojars.org/api/artifacts/prismic",
            body: r#"{"downloads": 4213, "latest_release": "7.0.0", "latest_version": "7.0.1-SNAPSHOT"}"#,
        };
        let mut p = params("prismic");
        p.insert("include_prereleases".to_string(), String::new());
        let value = resolve_version(&p, &fetcher).unwrap();
        assert_eq!(value, "[prismic \"7.0.1-SNAPSHOT\"]");
    }

    #[test]
    fn supports_group_slash_artifact_clojars() {
        let fetcher = FakeFetcher {
            expected_url: "https://clojars.org/api/artifacts/ring/ring-core",
            body: r#"{"downloads": 100, "latest_release": "1.0.0", "latest_version": "1.0.0"}"#,
        };
        let value = resolve_version(&params("ring/ring-core"), &fetcher).unwrap();
        assert_eq!(value, "[ring/ring-core \"1.0.0\"]");
    }

    #[test]
    fn requires_clojar_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid clojar")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid clojar")
            }
        }
        assert!(resolve_version(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_both_fields_are_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://clojars.org/api/artifacts/prismic",
            body: r#"{"downloads": 4213, "latest_release": null}"#,
        };
        assert!(resolve_version(&params("prismic"), &fetcher).is_err());
    }
}
