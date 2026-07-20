use super::validate_path_param;
use crate::Fetcher;
use std::collections::HashMap;

fn find_class_open_tag_end(html: &str, class_name: &str) -> Option<usize> {
    let mut search_from = 0;
    loop {
        let rel = html[search_from..].find("class=\"")?;
        let attr_start = search_from + rel + "class=\"".len();
        let attr_end = attr_start + html[attr_start..].find('"')?;
        let classes = &html[attr_start..attr_end];
        if classes.split_whitespace().any(|c| c == class_name) {
            let tag_end = attr_end + html[attr_end..].find('>')?;
            return Some(tag_end + 1);
        }
        search_from = attr_end + 1;
    }
}

fn text_immediately_after(html: &str, pos: usize) -> Option<String> {
    let rest = &html[pos..];
    let end = rest.find('<')?;
    let text = rest[..end].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

pub(crate) fn resolve_version(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let store_id = params
        .get("store-id")
        .ok_or("chrome-web-store-version requires a data-store-id attribute")?;
    let store_id = validate_path_param("store-id", store_id)?;

    let url = format!("https://chromewebstore.google.com/detail/{store_id}");
    let bytes = fetcher.fetch(&url)?;
    let html = String::from_utf8(bytes)
        .map_err(|_| "chrome web store response was not valid UTF-8".to_string())?;

    let pos = find_class_open_tag_end(&html, "nBZElf")
        .ok_or("chrome web store page did not contain a version element")?;
    text_immediately_after(&html, pos)
        .ok_or_else(|| "chrome web store version element was empty".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://chromewebstore.google.com/detail/ogffaloegjglncjfehdfplabnoondfjo"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(store_id: &str) -> HashMap<String, String> {
        HashMap::from([("store-id".to_string(), store_id.to_string())])
    }

    #[test]
    fn extracts_version_from_the_details_list() {
        let html = r#"<li class="MqICNe ecmXy"><div class="QDHp8e">Version</div><div class="nBZElf">1.2.7</div></li>"#;
        let value = resolve_version(
            &params("ogffaloegjglncjfehdfplabnoondfjo"),
            &FakeFetcher(html),
        )
        .unwrap();
        assert_eq!(value, "1.2.7");
    }

    #[test]
    fn requires_store_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid store id")
            }
        }
        assert!(resolve_version(&HashMap::new(), &Unused).is_err());
        assert!(resolve_version(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_store_id() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid store id")
            }
        }
        assert!(resolve_version(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_version_element_is_missing() {
        let html = r#"<div class="other">nothing here</div>"#;
        assert!(
            resolve_version(
                &params("ogffaloegjglncjfehdfplabnoondfjo"),
                &FakeFetcher(html)
            )
            .is_err()
        );
    }
}
