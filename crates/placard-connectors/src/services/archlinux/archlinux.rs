use super::validate_path_param;
use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

pub(crate) fn resolve_archlinux(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let repository = params
        .get("repository")
        .ok_or("archlinux requires a data-repository attribute")?;
    let architecture = params
        .get("architecture")
        .ok_or("archlinux requires a data-architecture attribute")?;
    let package_name = params
        .get("package-name")
        .ok_or("archlinux requires a data-package-name attribute")?;
    let repository = validate_path_param("repository", repository)?;
    let architecture = validate_path_param("architecture", architecture)?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!(
        "https://www.archlinux.org/packages/{repository}/{architecture}/{package_name}/json/"
    );
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "archlinux response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let pkgver = value
        .get("pkgver")
        .ok_or("archlinux response missing pkgver")?;
    pkgver
        .as_text()
        .ok_or_else(|| "pkgver was not a plain value".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://www.archlinux.org/packages/core/x86_64/pacman/json/"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(repository: &str, architecture: &str, package_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("repository".to_string(), repository.to_string()),
            ("architecture".to_string(), architecture.to_string()),
            ("package-name".to_string(), package_name.to_string()),
        ])
    }

    #[test]
    fn extracts_pkgver_from_an_archlinux_shaped_response() {
        let fetcher = FakeFetcher(r#"{"pkgname": "pacman", "pkgver": "6.1.0", "pkgrel": "2"}"#);
        let value = resolve_archlinux(&params("core", "x86_64", "pacman"), &fetcher).unwrap();
        assert_eq!(value, "6.1.0");
    }

    #[test]
    fn requires_all_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_archlinux(&HashMap::new(), &Unused).is_err());
        assert!(resolve_archlinux(&params("core", "x86_64", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_archlinux(&params("../etc", "x86_64", "pacman"), &Unused).is_err());
    }

    #[test]
    fn errors_when_pkgver_is_missing() {
        let fetcher = FakeFetcher(r#"{"pkgname": "pacman"}"#);
        assert!(resolve_archlinux(&params("core", "x86_64", "pacman"), &fetcher).is_err());
    }
}
