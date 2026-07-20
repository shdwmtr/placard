use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_debian(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("debian requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;
    let distribution = params
        .get("distribution")
        .map(String::as_str)
        .unwrap_or("stable");
    let distribution = validate_path_param("distribution", distribution)?;

    let url = format!(
        "https://api.ftp-master.debian.org/madison?f=json&s={distribution}&package={package}"
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "debian response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    let Value::Array(entries) = &value else {
        return Err("debian response was not an array".to_string());
    };
    let first = entries
        .first()
        .ok_or("debian response was empty (package not found)")?;
    let Value::Object(fields) = first else {
        return Err("debian response entry was not an object".to_string());
    };
    let package_data = fields
        .iter()
        .find(|(k, _)| k == package)
        .map(|(_, v)| v)
        .ok_or("debian response missing package data")?;
    let Value::Object(dist_fields) = package_data else {
        return Err("debian package data was not an object".to_string());
    };
    let (_, versions_value) = dist_fields
        .first()
        .ok_or("debian package data had no distributions")?;
    let Value::Object(version_fields) = versions_value else {
        return Err("debian distribution data was not an object".to_string());
    };
    let (version, _) = version_fields
        .first()
        .ok_or("debian distribution had no versions")?;
    Ok(version.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://api.ftp-master.debian.org/madison?f=json&s=stable&package=apt"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_the_first_version_from_a_debian_shaped_response() {
        let fetcher = FakeFetcher(r#"[{"apt": {"stable": {"2.6.1": {}}}}]"#);
        let value = resolve_debian(&params("apt"), &fetcher).unwrap();
        assert_eq!(value, "2.6.1");
    }

    #[test]
    fn uses_the_distribution_param_in_the_url() {
        struct FetcherAssertingUnstable;
        impl Fetcher for FetcherAssertingUnstable {
            fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
                assert_eq!(
                    url,
                    "https://api.ftp-master.debian.org/madison?f=json&s=unstable&package=apt"
                );
                Ok(r#"[{"apt": {"unstable": {"2.7.0": {}}}}]"#.as_bytes().to_vec())
            }
        }
        let mut p = params("apt");
        p.insert("distribution".to_string(), "unstable".to_string());
        let value = resolve_debian(&p, &FetcherAssertingUnstable).unwrap();
        assert_eq!(value, "2.7.0");
    }

    #[test]
    fn requires_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid param")
            }
        }
        assert!(resolve_debian(&HashMap::new(), &Unused).is_err());
        assert!(resolve_debian(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_debian(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_package_is_not_found() {
        let fetcher = FakeFetcher(r#"[]"#);
        assert!(resolve_debian(&params("apt"), &fetcher).is_err());
    }
}
