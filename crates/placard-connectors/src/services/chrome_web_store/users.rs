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

/// The user count sits as bare text at the end of the badge container
/// (after a couple of category `<a>` links), e.g.
/// `<div class="F9iKBc"><a ...>Extension</a><a ...>Category</a>608 users</div>`.
fn extract_users_count(html: &str) -> Option<String> {
    let open_end = find_class_open_tag_end(html, "F9iKBc")?;
    let close_rel = html[open_end..].find("</div>")?;
    let body = &html[open_end..open_end + close_rel];
    let idx = body.find(" users")?;
    let before = &body[..idx];
    let start = before
        .rfind(|c: char| !c.is_ascii_digit() && c != ',')
        .map(|i| i + 1)
        .unwrap_or(0);
    let digits = &before[start..];
    if digits.is_empty() {
        None
    } else {
        Some(digits.chars().filter(|c| *c != ',').collect())
    }
}

pub(crate) fn resolve_users(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let store_id = params
        .get("store-id")
        .ok_or("chrome-web-store-users requires a data-store-id attribute")?;
    let store_id = validate_path_param("store-id", store_id)?;

    let url = format!("https://chromewebstore.google.com/detail/{store_id}");
    let bytes = fetcher.fetch(&url)?;
    let html = String::from_utf8(bytes)
        .map_err(|_| "chrome web store response was not valid UTF-8".to_string())?;

    extract_users_count(&html)
        .ok_or_else(|| "chrome web store page did not contain a users element".to_string())
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
    fn extracts_the_user_count_trailing_text() {
        let html = r#"<div class="F9iKBc"><a class="gqpEIe FjUAcd" href="./category/extensions">Extension</a><a class="gqpEIe bgp7Ye" href="./category/x">Accessibility</a>608 users</div>"#;
        let value = resolve_users(
            &params("ogffaloegjglncjfehdfplabnoondfjo"),
            &FakeFetcher(html),
        )
        .unwrap();
        assert_eq!(value, "608");
    }

    #[test]
    fn strips_thousands_separators() {
        let html = r#"<div class="F9iKBc"><a>Extension</a>1,234,567 users</div>"#;
        let value = resolve_users(
            &params("ogffaloegjglncjfehdfplabnoondfjo"),
            &FakeFetcher(html),
        )
        .unwrap();
        assert_eq!(value, "1234567");
    }

    #[test]
    fn requires_store_id_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid store id")
            }
        }
        assert!(resolve_users(&HashMap::new(), &Unused).is_err());
        assert!(resolve_users(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_store_id() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid store id")
            }
        }
        assert!(resolve_users(&params("../etc"), &Unused).is_err());
    }

    #[test]
    fn errors_when_the_users_element_is_missing() {
        let html = r#"<div class="other">nothing here</div>"#;
        assert!(
            resolve_users(
                &params("ogffaloegjglncjfehdfplabnoondfjo"),
                &FakeFetcher(html)
            )
            .is_err()
        );
    }
}
