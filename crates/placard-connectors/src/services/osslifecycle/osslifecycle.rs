use crate::Fetcher;
use std::collections::HashMap;

fn validate_data_url(url: &str) -> Result<&str, String> {
    if url.is_empty() {
        return Err("'file_url' parameter must not be empty".to_string());
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(
            "'file_url' parameter must be a well-formed http:// or https:// URL".to_string(),
        );
    }
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(
            "'file_url' parameter contains disallowed whitespace or control characters".to_string(),
        );
    }
    Ok(url)
}

fn extract_status(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("osslifecycle=")?;
    let start = idx + "osslifecycle=".len();
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(rest[..end].to_ascii_lowercase())
}

pub(crate) fn resolve_osslifecycle(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let file_url = params
        .get("file_url")
        .ok_or("osslifecycle requires a data-file_url attribute")?;
    let file_url = validate_data_url(file_url)?;

    let bytes = fetcher.fetch(file_url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "osslifecycle response was not valid UTF-8".to_string())?;
    extract_status(&text).ok_or_else(|| "metadata in unexpected format".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(
                url,
                "https://raw.githubusercontent.com/Netflix/aws-autoscaling/master/OSSMETADATA"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(url: &str) -> HashMap<String, String> {
        HashMap::from([("file_url".to_string(), url.to_string())])
    }

    #[test]
    fn extracts_the_lifecycle_status() {
        let fetcher = FakeFetcher("osslifecycle=active\n");
        let value = resolve_osslifecycle(
            &params("https://raw.githubusercontent.com/Netflix/aws-autoscaling/master/OSSMETADATA"),
            &fetcher,
        )
        .unwrap();
        assert_eq!(value, "active");
    }

    #[test]
    fn requires_file_url_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid file_url")
            }
        }
        assert!(resolve_osslifecycle(&HashMap::new(), &Unused).is_err());
        assert!(resolve_osslifecycle(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        assert!(resolve_osslifecycle(&params("file:///etc/passwd"), &Unused).is_err());
    }

    #[test]
    fn errors_when_metadata_is_missing() {
        let fetcher = FakeFetcher("nothing to see here");
        assert!(
            resolve_osslifecycle(
                &params(
                    "https://raw.githubusercontent.com/Netflix/aws-autoscaling/master/OSSMETADATA"
                ),
                &fetcher
            )
            .is_err()
        );
    }
}
