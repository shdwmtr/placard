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

pub(crate) fn resolve_update(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let name = params
        .get("name")
        .ok_or("eclipse-marketplace-update requires a data-name attribute")?;
    let name = validate_path_param("name", name)?;

    let url = format!("https://marketplace.eclipse.org/content/{name}/api/p");
    let bytes = fetcher.fetch(&url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "eclipse marketplace response was not valid UTF-8".to_string())?;

    extract_xml_field(&text, "changed")
        .ok_or_else(|| "eclipse marketplace response missing changed".to_string())
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

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><marketplace><node id="595" name="Notepad4e"><favorited>10</favorited><installstotal>194941</installstotal><installsrecent>1234</installsrecent><license>Eclipse Public License</license><version>1.5.0</version><changed>1615000000</changed></node></marketplace>"#;

    #[test]
    fn extracts_changed_timestamp_from_an_eclipse_marketplace_shaped_response() {
        let fetcher = FakeFetcher(SAMPLE);
        let value = resolve_update(&params("notepad4e"), &fetcher).unwrap();
        assert_eq!(value, "1615000000");
    }

    #[test]
    fn requires_name_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_update(&HashMap::new(), &Unused).is_err());
        assert!(resolve_update(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        assert!(resolve_update(&params("../etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"<marketplace><node id="595"></node></marketplace>"#);
        assert!(resolve_update(&params("notepad4e"), &fetcher).is_err());
    }
}
