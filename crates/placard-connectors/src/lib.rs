mod format;
mod generic;
mod json;
mod services;

pub use services::all_presets;
pub use services::meta::{Param, PresetMeta, param_options};
pub use services::placard::PLACARDS_RENDERED_URL;

use placard_html::{Dom, NodeId};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub trait Fetcher: Send + Sync {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

pub struct CachingFetcher<F: Fetcher> {
    inner: F,
    ttl: Duration,
    cache: Mutex<HashMap<String, (Instant, Vec<u8>)>>,
}

impl<F: Fetcher> CachingFetcher<F> {
    pub fn new(inner: F, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl<F: Fetcher> Fetcher for CachingFetcher<F> {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        if let Some((fetched_at, bytes)) = self.cache.lock().unwrap().get(url) {
            if fetched_at.elapsed() < self.ttl {
                return Ok(bytes.clone());
            }
        }
        let bytes = self.inner.fetch(url)?;
        let mut cache = self.cache.lock().unwrap();
        let ttl = self.ttl;
        cache.retain(|_, (fetched_at, _)| fetched_at.elapsed() < ttl);
        cache.insert(url.to_string(), (Instant::now(), bytes.clone()));
        Ok(bytes)
    }
}

const MAX_CONNECTORS_PER_DOCUMENT: usize = 20;

pub fn resolve(dom: &mut Dom, fetcher: &dyn Fetcher) -> Vec<String> {
    let mut remaining = MAX_CONNECTORS_PER_DOCUMENT;
    let mut failures = Vec::new();
    resolve_node(dom, dom.root(), fetcher, &mut remaining, &mut failures);
    failures
}

fn resolve_node(
    dom: &mut Dom,
    node: NodeId,
    fetcher: &dyn Fetcher,
    remaining: &mut usize,
    failures: &mut Vec<String>,
) {
    if *remaining == 0 {
        return;
    }

    let mut resolved = false;
    let number_format = dom.attr(node, "data-number-format").map(str::to_string);

    if let Some(url) = dom.attr(node, "data-connector").map(str::to_string) {
        *remaining -= 1;
        match generic::resolve(&url, fetcher) {
            Ok(value) => {
                dom.set_text_content(node, &apply_number_format_if_present(value, &number_format));
                resolved = true;
            }
            Err(err) => {
                failures.push(format!(
                    "connector '{url}' failed, kept fallback content: {err}"
                ));
            }
        }
    } else if let Some(preset) = dom.attr(node, "data-preset").map(str::to_string) {
        *remaining -= 1;
        let params = collect_data_params(dom, node);
        match services::resolve_preset(&preset, &params, fetcher) {
            Ok(value) => {
                dom.set_text_content(node, &apply_number_format_if_present(value, &number_format));
                resolved = true;
            }
            Err(err) => {
                failures.push(format!(
                    "preset '{preset}' failed, kept fallback content: {err}"
                ));
            }
        }
    }

    if !resolved {
        let children: Vec<NodeId> = dom.children(node).collect();
        for child in children {
            resolve_node(dom, child, fetcher, remaining, failures);
        }
    }
}

fn apply_number_format_if_present(value: String, spec: &Option<String>) -> String {
    match spec {
        Some(spec) => format::apply_number_format(&value, spec).unwrap_or(value),
        None => value,
    }
}

fn collect_data_params(dom: &Dom, node: NodeId) -> HashMap<String, String> {
    dom.attrs(node)
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix("data-")
                .filter(|name| {
                    *name != "preset" && *name != "connector" && *name != "number-format"
                })
                .map(|name| (name.to_string(), v.clone()))
        })
        .collect()
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
    fn resolves_a_generic_connector_in_place() {
        let mut dom =
            placard_html::parse(r#"<span data-connector="https://example.com">fallback</span>"#);
        let fetcher = FakeFetcher("42");
        resolve(&mut dom, &fetcher);

        let span = dom.first_child(dom.root()).unwrap();
        let text_node = dom.first_child(span).unwrap();
        assert_eq!(dom.text(text_node), Some("42"));
    }

    #[test]
    fn resolves_a_github_stars_preset_using_data_attributes() {
        let mut dom = placard_html::parse(
            r#"<span data-preset="github-stars" data-owner="shdwmtr" data-repo="placard">0</span>"#,
        );
        let fetcher = FakeFetcher(r#"{"stargazers_count": 99}"#);
        resolve(&mut dom, &fetcher);

        let span = dom.first_child(dom.root()).unwrap();
        let text_node = dom.first_child(span).unwrap();
        assert_eq!(dom.text(text_node), Some("99"));
    }

    #[test]
    fn applies_data_number_format_to_a_resolved_preset_value() {
        let mut dom = placard_html::parse(
            r#"<span data-preset="github-stars" data-owner="shdwmtr" data-repo="placard" data-number-format="%,d">0</span>"#,
        );
        let fetcher = FakeFetcher(r#"{"stargazers_count": 12483}"#);
        resolve(&mut dom, &fetcher);

        let span = dom.first_child(dom.root()).unwrap();
        let text_node = dom.first_child(span).unwrap();
        assert_eq!(dom.text(text_node), Some("12,483"));
    }

    #[test]
    fn ignores_data_number_format_when_the_spec_is_malformed() {
        let mut dom = placard_html::parse(
            r#"<span data-preset="github-stars" data-owner="shdwmtr" data-repo="placard" data-number-format="not-a-spec">0</span>"#,
        );
        let fetcher = FakeFetcher(r#"{"stargazers_count": 12483}"#);
        resolve(&mut dom, &fetcher);

        let span = dom.first_child(dom.root()).unwrap();
        let text_node = dom.first_child(span).unwrap();
        assert_eq!(dom.text(text_node), Some("12483"));
    }

    #[test]
    fn does_not_leak_number_format_into_resolver_params() {
        let dom = placard_html::parse(
            r#"<div data-preset="x" data-number-format="%,d" data-owner="a"></div>"#,
        );
        let node = dom.first_child(dom.root()).unwrap();
        let params = collect_data_params(&dom, node);
        assert!(!params.contains_key("number-format"));
        assert_eq!(params.get("owner").map(String::as_str), Some("a"));
    }

    #[test]
    fn keeps_fallback_content_when_resolution_fails() {
        let mut dom =
            placard_html::parse(r#"<span data-connector="https://example.com">fallback</span>"#);
        struct FailingFetcher;
        impl Fetcher for FailingFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Err("network error".to_string())
            }
        }
        let failures = resolve(&mut dom, &FailingFetcher);

        let span = dom.first_child(dom.root()).unwrap();
        let text_node = dom.first_child(span).unwrap();
        assert_eq!(dom.text(text_node), Some("fallback"));
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("https://example.com"));
        assert!(failures[0].contains("network error"));
    }

    #[test]
    fn reports_the_preset_name_when_a_preset_fails() {
        let mut dom = placard_html::parse(
            r#"<span data-preset="github-stars" data-owner="shdwmtr" data-repo="placard">0</span>"#,
        );
        struct FailingFetcher;
        impl Fetcher for FailingFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                Err("rate limited".to_string())
            }
        }
        let failures = resolve(&mut dom, &FailingFetcher);

        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("github-stars"));
        assert!(failures[0].contains("rate limited"));
    }

    #[test]
    fn returns_no_failures_when_everything_resolves() {
        let mut dom = placard_html::parse(
            r#"<span data-preset="github-stars" data-owner="shdwmtr" data-repo="placard">0</span>"#,
        );
        let fetcher = FakeFetcher(r#"{"stargazers_count": 99}"#);
        let failures = resolve(&mut dom, &fetcher);
        assert!(failures.is_empty());
    }

    #[test]
    fn resolves_connectors_nested_deep_in_the_tree() {
        let mut dom = placard_html::parse(
            r#"<div><p><span data-connector="https://example.com">x</span></p></div>"#,
        );
        let fetcher = FakeFetcher("nested-value");
        resolve(&mut dom, &fetcher);

        let div = dom.first_child(dom.root()).unwrap();
        let p = dom.first_child(div).unwrap();
        let span = dom.first_child(p).unwrap();
        let text_node = dom.first_child(span).unwrap();
        assert_eq!(dom.text(text_node), Some("nested-value"));
    }

    #[test]
    fn caching_fetcher_only_calls_inner_once_per_url_within_the_ttl() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingFetcher(AtomicUsize);
        impl Fetcher for CountingFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(b"value".to_vec())
            }
        }

        let counting = CountingFetcher(AtomicUsize::new(0));
        let cached = CachingFetcher::new(counting, Duration::from_secs(60));

        cached.fetch("https://example.com").unwrap();
        cached.fetch("https://example.com").unwrap();
        cached.fetch("https://example.com").unwrap();

        assert_eq!(cached.inner.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn stops_resolving_connectors_past_the_per_document_cap() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingFetcher(AtomicUsize);
        impl Fetcher for CountingFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(b"value".to_vec())
            }
        }

        let html: String = (0..MAX_CONNECTORS_PER_DOCUMENT * 2)
            .map(|i| format!(r#"<span data-connector="https://example.com/{i}">x</span>"#))
            .collect();
        let mut dom = placard_html::parse(&html);
        let fetcher = CountingFetcher(AtomicUsize::new(0));
        resolve(&mut dom, &fetcher);

        assert_eq!(
            fetcher.0.load(Ordering::SeqCst),
            MAX_CONNECTORS_PER_DOCUMENT
        );
    }

    #[test]
    fn caching_fetcher_refetches_different_urls_independently() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingFetcher(AtomicUsize);
        impl Fetcher for CountingFetcher {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(b"value".to_vec())
            }
        }

        let counting = CountingFetcher(AtomicUsize::new(0));
        let cached = CachingFetcher::new(counting, Duration::from_secs(60));

        cached.fetch("https://example.com/a").unwrap();
        cached.fetch("https://example.com/b").unwrap();

        assert_eq!(cached.inner.0.load(Ordering::SeqCst), 2);
    }
}
