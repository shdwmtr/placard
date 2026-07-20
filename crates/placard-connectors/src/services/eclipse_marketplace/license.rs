use super::super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn extract_xml_field(xml: &str, tag: &str) -> Option<String> {
    let self_closing = format!("<{tag}/>");
    if xml.contains(&self_closing) {
        return Some(String::new());
    }
    let self_closing_spaced = format!("<{tag} />");
    if xml.contains(&self_closing_spaced) {
        return Some(String::new());
    }
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let inner = xml[start..end].trim();
    let inner = inner
        .strip_prefix("<![CDATA[")
        .and_then(|s| s.strip_suffix("]]>"))
        .unwrap_or(inner)
        .trim();
    Some(decode_xml_entities(inner))
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

pub(crate) fn resolve_license(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let name = params
        .get("name")
        .ok_or("eclipse-marketplace-license requires a data-name attribute")?;
    let name = validate_path_param("name", name)?;

    let url = format!("https://marketplace.eclipse.org/content/{name}/api/p");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "eclipse marketplace response was not valid UTF-8".to_string())?;

    let license = extract_xml_field(&text, "license")
        .ok_or_else(|| "eclipse marketplace response missing license".to_string())?;
    if license.is_empty() {
        Ok("not specified".to_string())
    } else {
        Ok(license)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://marketplace.eclipse.org/content/notepad4e/api/p"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(name: &str) -> HashMap<String, String> {
        HashMap::from([("name".to_string(), name.to_string())])
    }

    #[test]
    fn extracts_license_from_an_eclipse_marketplace_shaped_response() {
        let fetcher = FakeFetcher(
            r#"<marketplace><node id="595"><license>Eclipse Public License</license></node></marketplace>"#,
        );
        let value = resolve_license(&params("notepad4e"), &fetcher).unwrap();
        assert_eq!(value, "Eclipse Public License");
    }

    #[test]
    fn returns_not_specified_for_an_empty_license() {
        let fetcher =
            FakeFetcher(r#"<marketplace><node id="595"><license></license></node></marketplace>"#);
        let value = resolve_license(&params("notepad4e"), &fetcher).unwrap();
        assert_eq!(value, "not specified");
    }

    #[test]
    fn requires_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
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
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"<marketplace><node id="595"></node></marketplace>"#);
        assert!(resolve_license(&params("notepad4e"), &fetcher).is_err());
    }
}
