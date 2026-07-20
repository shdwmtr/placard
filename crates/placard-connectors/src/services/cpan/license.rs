use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package = params
        .get("package")
        .ok_or("cpan-license requires a data-package attribute")?;
    let package = validate_path_param("package", package)?;

    let url = format!("https://fastapi.metacpan.org/v1/release/{package}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "cpan response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let license = value
        .get("license")
        .ok_or("cpan response missing license")?;
    match license {
        Value::Array(items) => items
            .first()
            .and_then(|v| v.as_text())
            .ok_or_else(|| "cpan license array was empty".to_string()),
        _ => Err("cpan license field was not an array".to_string()),
    }
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

    fn params(package: &str) -> HashMap<String, String> {
        HashMap::from([("package".to_string(), package.to_string())])
    }

    #[test]
    fn extracts_the_first_license() {
        let fetcher = FakeFetcher {
            expected_url: "https://fastapi.metacpan.org/v1/release/Config-Augeas",
            body: r#"{"version": "1.0", "license": ["perl_5", "mit"]}"#,
        };
        let value = resolve_license(&params("Config-Augeas"), &fetcher).unwrap();
        assert_eq!(value, "perl_5");
    }

    #[test]
    fn requires_a_package_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid package")
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
    fn errors_when_the_field_is_missing_or_empty() {
        let fetcher = FakeFetcher {
            expected_url: "https://fastapi.metacpan.org/v1/release/Config-Augeas",
            body: r#"{"version": "1.0"}"#,
        };
        assert!(resolve_license(&params("Config-Augeas"), &fetcher).is_err());

        let fetcher_empty = FakeFetcher {
            expected_url: "https://fastapi.metacpan.org/v1/release/Config-Augeas",
            body: r#"{"version": "1.0", "license": []}"#,
        };
        assert!(resolve_license(&params("Config-Augeas"), &fetcher_empty).is_err());
    }
}
