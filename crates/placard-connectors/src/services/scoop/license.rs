use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

fn parse_github_repo_url(url: &str) -> Option<(String, String)> {
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    let mut parts = rest.split('/').filter(|s| !s.is_empty());
    let user = parts.next()?;
    let repo = parts.next()?;
    Some((user.to_string(), repo.to_string()))
}

/// Scoop's bucket registry maps a short bucket name (default `main`) to the
/// GitHub repo that hosts it. A `bucket` param that isn't a known name is
/// tried as a literal `https://github.com/<user>/<repo>` URL instead, same
/// as shields does.
fn resolve_bucket_repo(
    bucket_param: Option<&str>,
    fetcher: &dyn Fetcher,
) -> Result<(String, String), String> {
    let bytes = fetcher
        .fetch("https://raw.githubusercontent.com/ScoopInstaller/Scoop/master/buckets.json")?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "scoop buckets response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let bucket = bucket_param.filter(|b| !b.is_empty()).unwrap_or("main");

    let bucket_url = match &value {
        Value::Object(fields) => fields
            .iter()
            .find(|(k, _)| k == bucket)
            .and_then(|(_, v)| v.as_text()),
        _ => None,
    };

    let repo_url = bucket_url.unwrap_or_else(|| bucket.to_string());
    parse_github_repo_url(&repo_url).ok_or_else(|| format!("bucket \"{bucket}\" not found"))
}

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let app = params
        .get("app")
        .ok_or("scoop-license requires a data-app attribute")?;
    let app = validate_path_param("app", app)?;
    let bucket_param = params.get("bucket").map(String::as_str);

    let (user, repo) = resolve_bucket_repo(bucket_param, fetcher)?;
    let user = validate_path_param("user", &user)?;
    let repo = validate_path_param("repo", &repo)?;

    let url = format!("https://raw.githubusercontent.com/{user}/{repo}/master/bucket/{app}.json");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "scoop app response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let license = value
        .get("license")
        .ok_or("scoop app response missing license")?;

    match license {
        Value::String(s) => Ok(s.clone()),
        Value::Object(fields) => fields
            .iter()
            .find(|(k, _)| k == "identifier")
            .and_then(|(_, v)| v.as_text())
            .ok_or_else(|| "scoop app response license missing identifier".to_string()),
        _ => Err("scoop app response license was not a plain value".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeFetcher {
        buckets_response: &'static str,
        app_response: &'static str,
        expected_app_url: &'static str,
        calls: AtomicUsize,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => {
                    assert_eq!(
                        url,
                        "https://raw.githubusercontent.com/ScoopInstaller/Scoop/master/buckets.json"
                    );
                    Ok(self.buckets_response.as_bytes().to_vec())
                }
                1 => {
                    assert_eq!(url, self.expected_app_url);
                    Ok(self.app_response.as_bytes().to_vec())
                }
                _ => panic!("unexpected extra fetch"),
            }
        }
    }

    const BUCKETS: &str = r#"{"main": "https://github.com/ScoopInstaller/Main", "extras": "https://github.com/ScoopInstaller/Extras"}"#;

    fn params(app: &str) -> HashMap<String, String> {
        HashMap::from([("app".to_string(), app.to_string())])
    }

    #[test]
    fn extracts_a_plain_string_license_from_the_default_bucket() {
        let fetcher = FakeFetcher {
            buckets_response: BUCKETS,
            app_response: r#"{"version": "3.6.0", "license": "MIT"}"#,
            expected_app_url: "https://raw.githubusercontent.com/ScoopInstaller/Main/master/bucket/ngrok.json",
            calls: AtomicUsize::new(0),
        };
        let value = resolve_license(&params("ngrok"), &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn extracts_an_object_license_identifier() {
        let fetcher = FakeFetcher {
            buckets_response: BUCKETS,
            app_response: r#"{"version": "1.0.0", "license": {"identifier": "Apache-2.0"}}"#,
            expected_app_url: "https://raw.githubusercontent.com/ScoopInstaller/Main/master/bucket/ngrok.json",
            calls: AtomicUsize::new(0),
        };
        let value = resolve_license(&params("ngrok"), &fetcher).unwrap();
        assert_eq!(value, "Apache-2.0");
    }

    #[test]
    fn uses_the_bucket_param_to_pick_a_different_repo() {
        let fetcher = FakeFetcher {
            buckets_response: BUCKETS,
            app_response: r#"{"version": "1.0.0", "license": "MIT"}"#,
            expected_app_url: "https://raw.githubusercontent.com/ScoopInstaller/Extras/master/bucket/example.json",
            calls: AtomicUsize::new(0),
        };
        let mut p = params("example");
        p.insert("bucket".to_string(), "extras".to_string());
        let value = resolve_license(&p, &fetcher).unwrap();
        assert_eq!(value, "MIT");
    }

    #[test]
    fn requires_app_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid app")
            }
        }
        assert!(resolve_license(&HashMap::new(), &Unused).is_err());
        assert!(resolve_license(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_license(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_bucket_is_unknown() {
        let fetcher = FakeFetcher {
            buckets_response: BUCKETS,
            app_response: "",
            expected_app_url: "unused",
            calls: AtomicUsize::new(0),
        };
        let mut p = params("ngrok");
        p.insert("bucket".to_string(), "not-a-real-bucket".to_string());
        assert!(resolve_license(&p, &fetcher).is_err());
    }
}
