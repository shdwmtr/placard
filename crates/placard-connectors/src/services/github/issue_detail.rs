use super::validate_path_param;
use crate::Fetcher;
use crate::json::{self, Value};
use std::collections::HashMap;

const PROPERTIES: &[&str] = &[
    "state",
    "title",
    "author",
    "comments",
    "milestone",
    "age",
    "last-update",
    "label",
];

pub(crate) fn resolve_issue_detail(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let owner = params
        .get("owner")
        .ok_or("github-issue-detail requires a data-owner attribute")?;
    let repo = params
        .get("repo")
        .ok_or("github-issue-detail requires a data-repo attribute")?;
    let number = params
        .get("number")
        .ok_or("github-issue-detail requires a data-number attribute")?;
    let property = params
        .get("property")
        .ok_or("github-issue-detail requires a data-property attribute")?;
    let owner = validate_path_param("owner", owner)?;
    let repo = validate_path_param("repo", repo)?;
    let number = validate_path_param("number", number)?;
    if !PROPERTIES.contains(&property.as_str()) {
        return Err(format!(
            "'{property}' is not a supported github-issue-detail property"
        ));
    }

    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{number}");
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "github response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;

    match property.as_str() {
        "state" => value
            .get("state")
            .and_then(Value::as_text)
            .ok_or_else(|| "issue response missing state".to_string()),
        "title" => value
            .get("title")
            .and_then(Value::as_text)
            .ok_or_else(|| "issue response missing title".to_string()),
        "author" => value
            .get("user.login")
            .and_then(Value::as_text)
            .ok_or_else(|| "issue response missing user.login".to_string()),
        "comments" => value
            .get("comments")
            .and_then(Value::as_text)
            .ok_or_else(|| "issue response missing comments".to_string()),
        "milestone" => value
            .get("milestone.title")
            .and_then(Value::as_text)
            .ok_or_else(|| "issue response has no milestone".to_string()),
        "age" => value
            .get("created_at")
            .and_then(Value::as_text)
            .ok_or_else(|| "issue response missing created_at".to_string()),
        "last-update" => value
            .get("updated_at")
            .and_then(Value::as_text)
            .ok_or_else(|| "issue response missing updated_at".to_string()),
        "label" => match value.get("labels") {
            Some(Value::Array(items)) if !items.is_empty() => {
                let names: Option<Vec<String>> = items
                    .iter()
                    .map(|item| item.get("name").and_then(Value::as_text))
                    .collect();
                names
                    .map(|names| names.join(" | "))
                    .ok_or_else(|| "issue response has a label missing a name".to_string())
            }
            _ => Err("issue response has no labels".to_string()),
        },
        _ => unreachable!("property was validated against PROPERTIES above"),
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
                "https://api.github.com/repos/badges/shields/issues/979"
            );
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(property: &str) -> HashMap<String, String> {
        HashMap::from([
            ("owner".to_string(), "badges".to_string()),
            ("repo".to_string(), "shields".to_string()),
            ("number".to_string(), "979".to_string()),
            ("property".to_string(), property.to_string()),
        ])
    }

    #[test]
    fn extracts_state() {
        let fetcher = FakeFetcher(r#"{"state": "open", "number": 979}"#);
        assert_eq!(
            resolve_issue_detail(&params("state"), &fetcher).unwrap(),
            "open"
        );
    }

    #[test]
    fn extracts_title() {
        let fetcher = FakeFetcher(r#"{"title": "Something broke"}"#);
        assert_eq!(
            resolve_issue_detail(&params("title"), &fetcher).unwrap(),
            "Something broke"
        );
    }

    #[test]
    fn extracts_author_login() {
        let fetcher = FakeFetcher(r#"{"user": {"login": "paulmelnikow"}}"#);
        assert_eq!(
            resolve_issue_detail(&params("author"), &fetcher).unwrap(),
            "paulmelnikow"
        );
    }

    #[test]
    fn extracts_comments_count() {
        let fetcher = FakeFetcher(r#"{"comments": 12}"#);
        assert_eq!(
            resolve_issue_detail(&params("comments"), &fetcher).unwrap(),
            "12"
        );
    }

    #[test]
    fn extracts_milestone_title() {
        let fetcher = FakeFetcher(r#"{"milestone": {"title": "v2.0"}}"#);
        assert_eq!(
            resolve_issue_detail(&params("milestone"), &fetcher).unwrap(),
            "v2.0"
        );
    }

    #[test]
    fn extracts_age_from_created_at() {
        let fetcher = FakeFetcher(r#"{"created_at": "2020-01-01T00:00:00Z"}"#);
        assert_eq!(
            resolve_issue_detail(&params("age"), &fetcher).unwrap(),
            "2020-01-01T00:00:00Z"
        );
    }

    #[test]
    fn extracts_last_update_from_updated_at() {
        let fetcher = FakeFetcher(r#"{"updated_at": "2020-06-01T00:00:00Z"}"#);
        assert_eq!(
            resolve_issue_detail(&params("last-update"), &fetcher).unwrap(),
            "2020-06-01T00:00:00Z"
        );
    }

    #[test]
    fn extracts_joined_label_names() {
        let fetcher = FakeFetcher(
            r#"{"labels": [{"name": "bug", "color": "red"}, {"name": "help wanted", "color": "blue"}]}"#,
        );
        assert_eq!(
            resolve_issue_detail(&params("label"), &fetcher).unwrap(),
            "bug | help wanted"
        );
    }

    #[test]
    fn requires_owner_repo_number_and_property_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_issue_detail(&HashMap::new(), &Unused).is_err());
        let mut p = params("state");
        p.remove("number");
        assert!(resolve_issue_detail(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_an_unsupported_property() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid property")
            }
        }
        assert!(resolve_issue_detail(&params("bogus"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_params_before_fetching() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid param")
            }
        }
        let mut p = params("state");
        p.insert("owner".to_string(), "../etc".to_string());
        assert!(resolve_issue_detail(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher(r#"{"number": 979}"#);
        assert!(resolve_issue_detail(&params("state"), &fetcher).is_err());
    }
}
