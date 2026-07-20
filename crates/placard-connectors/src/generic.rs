use crate::Fetcher;

pub(crate) fn resolve(url: &str, fetcher: &dyn Fetcher) -> Result<String, String> {
    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "connector response was not valid UTF-8".to_string())?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
            Ok(self.0.as_bytes().to_vec())
        }
    }

    #[test]
    fn trims_whitespace_from_the_response_body() {
        let fetcher = FakeFetcher("  12,483 downloads  \n");
        assert_eq!(
            resolve("https://example.com", &fetcher).unwrap(),
            "12,483 downloads"
        );
    }

    struct FailingFetcher;
    impl Fetcher for FailingFetcher {
        fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
            Err("connection refused".to_string())
        }
    }

    #[test]
    fn propagates_fetch_errors() {
        assert!(resolve("https://example.com", &FailingFetcher).is_err());
    }
}
