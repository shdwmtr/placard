use super::super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

pub(crate) fn resolve_module_pdk_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let user = params
        .get("user")
        .ok_or("puppetforge-module-pdk-version requires a data-user attribute")?;
    let module_name = params
        .get("module-name")
        .ok_or("puppetforge-module-pdk-version requires a data-module-name attribute")?;
    let user = validate_path_param("user", user)?;
    let module_name = validate_path_param("module-name", module_name)?;

    let url = format!("https://forgeapi.puppetlabs.com/v3/modules/{user}-{module_name}");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "puppetforge response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let is_pdk = matches!(value.get("current_release.pdk"), Some(Value::Bool(true)));
    if !is_pdk {
        return Err("puppetforge module has no pdk version".to_string());
    }
    value
        .get("current_release.metadata.pdk-version")
        .and_then(Value::as_text)
        .ok_or_else(|| {
            "puppetforge response missing current_release.metadata.pdk-version".to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://forgeapi.puppetlabs.com/v3/modules/tragiccode-azure_key_vault"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(user: &str, module_name: &str) -> HashMap<String, String> {
        HashMap::from([
            ("user".to_string(), user.to_string()),
            ("module-name".to_string(), module_name.to_string()),
        ])
    }

    #[test]
    fn extracts_pdk_version_when_pdk_is_true() {
        let fetcher = FakeFetcher(
            r#"{"current_release": {"pdk": true, "version": "1.0.0", "metadata": {"pdk-version": "1.18.1"}}}"#,
        );
        let value =
            resolve_module_pdk_version(&params("tragiccode", "azure_key_vault"), &fetcher).unwrap();
        assert_eq!(value, "1.18.1");
    }

    #[test]
    fn requires_user_and_module_name_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_module_pdk_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_module_pdk_version(&params("tragiccode", ""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_module_pdk_version(&params("../etc", "azure_key_vault"), &Unused).is_err());
    }

    #[test]
    fn errors_when_pdk_is_false() {
        let fetcher = FakeFetcher(r#"{"current_release": {"pdk": false, "version": "1.0.0"}}"#);
        assert!(
            resolve_module_pdk_version(&params("tragiccode", "azure_key_vault"), &fetcher).is_err()
        );
    }
}
