use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn extract_version(cabal: &str) -> Option<String> {
    cabal
        .lines()
        .find(|line| {
            line.trim_start()
                .to_ascii_lowercase()
                .starts_with("version:")
        })
        .and_then(|line| line.splitn(2, ':').nth(1))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let package_name = params
        .get("package-name")
        .ok_or("hackage-version requires a data-package-name attribute")?;
    let package_name = validate_path_param("package-name", package_name)?;

    let url = format!("https://hackage.haskell.org/package/{package_name}/{package_name}.cabal");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "hackage response was not valid UTF-8".to_string())?;
    extract_version(&text).ok_or_else(|| "hackage cabal file missing a version field".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://hackage.haskell.org/package/lens/lens.cabal");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(package_name: &str) -> HashMap<String, String> {
        HashMap::from([("package-name".to_string(), package_name.to_string())])
    }

    #[test]
    fn extracts_version_from_a_cabal_file() {
        let fetcher = FakeFetcher("name: lens\nversion: 5.2.3\nbuild-type: Simple\n");
        let value = resolve_version(&params("lens"), &fetcher).unwrap();
        assert_eq!(value, "5.2.3");
    }

    #[test]
    fn requires_package_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
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
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_version(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_version_field_is_missing() {
        let fetcher = FakeFetcher("name: lens\nbuild-type: Simple\n");
        assert!(resolve_version(&params("lens"), &fetcher).is_err());
    }
}
