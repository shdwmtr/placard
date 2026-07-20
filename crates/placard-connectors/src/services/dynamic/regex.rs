use crate::Fetcher;
use std::collections::HashMap;

const VALID_FLAGS: &str = "ims";

fn validate_data_url(url: &str) -> Result<&str, String> {
    if url.is_empty() {
        return Err("'url' parameter must not be empty".to_string());
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("'url' parameter must be a well-formed http:// or https:// URL".to_string());
    }
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(
            "'url' parameter contains disallowed whitespace or control characters".to_string(),
        );
    }
    Ok(url)
}

// This is a deliberately small subset of shields' re2-based `data-search`
// syntax: only literal text and wildcard capture groups written as
// `(.*)`, `(.+)`, `(.*?)`, or `(.+?)` are understood (all four are treated
// as "consume characters up to the next literal segment, or to the end of
// the string if there is none"). Character classes, alternation, anchors,
// quantifiers on literal text, and named groups are not supported -- a
// pattern using any of that is rejected with a clear error rather than
// silently matching something else.
#[derive(Debug, PartialEq)]
enum Token {
    Literal(String),
    Capture,
}

fn parse_pattern(search: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = search.chars().collect();
    let mut tokens = Vec::new();
    let mut literal = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '(' {
            let close = chars[i..]
                .iter()
                .position(|&c| c == ')')
                .map(|p| p + i)
                .ok_or("unterminated capture group in search pattern")?;
            let inner: String = chars[i + 1..close].iter().collect();
            if !matches!(inner.as_str(), ".*" | ".+" | ".*?" | ".+?") {
                return Err(format!(
                    "unsupported regex construct '({inner})'; only literal text and wildcard capture groups like (.*) are supported"
                ));
            }
            if !literal.is_empty() {
                tokens.push(Token::Literal(std::mem::take(&mut literal)));
            }
            if matches!(tokens.last(), Some(Token::Capture)) {
                return Err(
                    "adjacent capture groups with no literal text between them are not supported"
                        .to_string(),
                );
            }
            tokens.push(Token::Capture);
            i = close + 1;
        } else {
            literal.push(chars[i]);
            i += 1;
        }
    }
    if !literal.is_empty() {
        tokens.push(Token::Literal(literal));
    }
    if tokens.is_empty() {
        return Err("'search' parameter must not be empty".to_string());
    }
    Ok(tokens)
}

struct Match {
    full: String,
    groups: Vec<String>,
}

fn chars_eq(a: &[char], b: &[char], case_insensitive: bool) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| {
        if case_insensitive {
            x.eq_ignore_ascii_case(y)
        } else {
            x == y
        }
    })
}

fn try_match_at(
    hay: &[char],
    start: usize,
    tokens: &[Token],
    case_insensitive: bool,
) -> Option<Match> {
    let mut pos = start;
    let mut groups = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::Literal(lit) => {
                let lit_chars: Vec<char> = lit.chars().collect();
                if pos + lit_chars.len() > hay.len() {
                    return None;
                }
                if !chars_eq(
                    &hay[pos..pos + lit_chars.len()],
                    &lit_chars,
                    case_insensitive,
                ) {
                    return None;
                }
                pos += lit_chars.len();
            }
            Token::Capture => {
                let next_literal = tokens[i + 1..].iter().find_map(|t| match t {
                    Token::Literal(l) => Some(l.chars().collect::<Vec<char>>()),
                    Token::Capture => None,
                });
                match next_literal {
                    Some(lit_chars) => {
                        let mut j = pos;
                        let mut found = None;
                        while j + lit_chars.len() <= hay.len() {
                            if chars_eq(&hay[j..j + lit_chars.len()], &lit_chars, case_insensitive)
                            {
                                found = Some(j);
                                break;
                            }
                            j += 1;
                        }
                        let idx = found?;
                        groups.push(hay[pos..idx].iter().collect());
                        pos = idx;
                    }
                    None => {
                        groups.push(hay[pos..].iter().collect());
                        pos = hay.len();
                    }
                }
            }
        }
    }

    Some(Match {
        full: hay[start..pos].iter().collect(),
        groups,
    })
}

fn find_match(haystack: &str, tokens: &[Token], case_insensitive: bool) -> Option<Match> {
    let hay: Vec<char> = haystack.chars().collect();
    (0..=hay.len()).find_map(|start| try_match_at(&hay, start, tokens, case_insensitive))
}

fn apply_replace(template: &str, groups: &[String]) -> Result<String, String> {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' {
            if i + 1 < chars.len() && chars[i + 1] == '$' {
                out.push('$');
                i += 2;
                continue;
            }
            let mut j = i + 1;
            let mut num = String::new();
            while j < chars.len() && chars[j].is_ascii_digit() {
                num.push(chars[j]);
                j += 1;
            }
            if !num.is_empty() {
                let idx: usize = num.parse().unwrap();
                if idx == 0 || idx > groups.len() {
                    return Err(format!(
                        "replace references group ${idx} but only {} group(s) were captured",
                        groups.len()
                    ));
                }
                out.push_str(&groups[idx - 1]);
                i = j;
                continue;
            }
            return Err(
                "replace string has a '$' not followed by a digit or another '$'".to_string(),
            );
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
}

pub(crate) fn resolve_regex(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let url = params
        .get("url")
        .ok_or("dynamic-regex requires a data-url attribute")?;
    let url = validate_data_url(url)?;
    let search = params
        .get("search")
        .ok_or("dynamic-regex requires a data-search attribute")?;
    if search.is_empty() {
        return Err("'search' parameter must not be empty".to_string());
    }
    if let Some(flags) = params.get("flags")
        && flags.chars().any(|c| !VALID_FLAGS.contains(c))
    {
        return Err(format!(
            "'flags' parameter must only contain characters from '{VALID_FLAGS}'"
        ));
    }
    let case_insensitive = params
        .get("flags")
        .map(|f| f.contains('i'))
        .unwrap_or(false);

    let tokens = parse_pattern(search)?;

    let bytes = fetcher.fetch(url)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "dynamic-regex response was not valid UTF-8".to_string())?;

    let found = find_match(&text, &tokens, case_insensitive).ok_or("no result")?;

    match params.get("replace") {
        Some(replace) => apply_replace(replace, &found.groups),
        None => Ok(found.full),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher(&'static str);
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, "https://example.com/README.md");
            Ok(self.0.as_bytes().to_vec())
        }
    }

    fn params(search: &str) -> HashMap<String, String> {
        HashMap::from([
            (
                "url".to_string(),
                "https://example.com/README.md".to_string(),
            ),
            ("search".to_string(), search.to_string()),
        ])
    }

    #[test]
    fn extracts_full_match_with_no_replace() {
        let fetcher = FakeFetcher("build status: passing on main");
        let value = resolve_regex(&params("status: (.*?) on"), &fetcher).unwrap();
        assert_eq!(value, "status: passing on");
    }

    #[test]
    fn applies_replace_template_with_capture_group() {
        let fetcher = FakeFetcher("version - 2.4 released");
        let mut p = params("version - (.*?) released");
        p.insert("replace".to_string(), "$1".to_string());
        let value = resolve_regex(&p, &fetcher).unwrap();
        assert_eq!(value, "2.4");
    }

    #[test]
    fn is_case_insensitive_with_i_flag() {
        let fetcher = FakeFetcher("Every DAY it serves 42 images");
        let mut p = params("every (.*?) it serves (.*?) images");
        p.insert("replace".to_string(), "$1/$2".to_string());
        p.insert("flags".to_string(), "i".to_string());
        let value = resolve_regex(&p, &fetcher).unwrap();
        assert_eq!(value, "DAY/42");
    }

    #[test]
    fn requires_url_and_search_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without valid params")
            }
        }
        assert!(resolve_regex(&HashMap::new(), &Unused).is_err());
        assert!(resolve_regex(&params(""), &Unused).is_err());
    }

    #[test]
    fn rejects_non_http_urls() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid url scheme")
            }
        }
        let mut p = params("foo");
        p.insert("url".to_string(), "file:///etc/passwd".to_string());
        assert!(resolve_regex(&p, &Unused).is_err());
    }

    #[test]
    fn rejects_unsupported_regex_constructs() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid pattern")
            }
        }
        assert!(resolve_regex(&params(r"version (\d+)"), &Unused).is_err());
        assert!(resolve_regex(&params("(a)(b)"), &Unused).is_err());
    }

    #[test]
    fn errors_when_no_match_is_found() {
        let fetcher = FakeFetcher("nothing relevant here");
        assert!(resolve_regex(&params("version - (.*?) released"), &fetcher).is_err());
    }
}
