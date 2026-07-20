use crate::Fetcher;
use crate::json;
use std::collections::HashMap;

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn host_from_alias(room_alias: &str) -> Result<String, String> {
    let parts: Vec<&str> = room_alias.split(':').collect();
    match parts.len() {
        2 => Ok(parts[1].to_string()),
        3 => Ok(format!("{}:{}", parts[1], parts[2])),
        _ => Err("'room-alias' parameter must be in the form localpart:server".to_string()),
    }
}

pub(crate) fn resolve_matrix(
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    let room_alias = params
        .get("room-alias")
        .filter(|v| !v.is_empty())
        .ok_or("matrix requires a data-room-alias attribute")?;

    let host = match params.get("server_fqdn") {
        Some(v) if !v.is_empty() => v.clone(),
        _ => host_from_alias(room_alias)?,
    };
    let host = validate_host(&host)?;

    let url = format!(
        "https://{host}/_matrix/client/unstable/im.nheko.summary/rooms/%23{}/summary",
        percent_encode(room_alias)
    );
    let bytes = fetcher.fetch(&url)?;
    let text =
        String::from_utf8(bytes).map_err(|_| "matrix response was not valid UTF-8".to_string())?;
    let value = json::parse(&text)?;
    let members = value
        .get("num_joined_members")
        .ok_or("matrix response missing num_joined_members")?;
    members
        .as_text()
        .ok_or_else(|| "num_joined_members was not a plain value".to_string())
}

fn validate_host(host: &str) -> Result<String, String> {
    if host.is_empty() {
        return Err("'server_fqdn' parameter must not be empty".to_string());
    }
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == ':')
    {
        return Err("'server_fqdn' parameter contains disallowed characters".to_string());
    }
    Ok(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        expected_url: &'static str,
        body: &'static str,
    }
    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            assert_eq!(url, self.expected_url);
            Ok(self.body.as_bytes().to_vec())
        }
    }

    fn params(room_alias: &str) -> HashMap<String, String> {
        HashMap::from([("room-alias".to_string(), room_alias.to_string())])
    }

    #[test]
    fn derives_the_host_from_the_room_alias() {
        let fetcher = FakeFetcher {
            expected_url: "https://matrix.org/_matrix/client/unstable/im.nheko.summary/rooms/%23twim%3Amatrix.org/summary",
            body: r#"{"num_joined_members": 250}"#,
        };
        let value = resolve_matrix(&params("twim:matrix.org"), &fetcher).unwrap();
        assert_eq!(value, "250");
    }

    #[test]
    fn uses_an_explicit_server_fqdn_when_provided() {
        let mut p = params("mysuperroom:example.com");
        p.insert("server_fqdn".to_string(), "matrix.example.com".to_string());
        let fetcher = FakeFetcher {
            expected_url: "https://matrix.example.com/_matrix/client/unstable/im.nheko.summary/rooms/%23mysuperroom%3Aexample.com/summary",
            body: r#"{"num_joined_members": 12}"#,
        };
        let value = resolve_matrix(&p, &fetcher).unwrap();
        assert_eq!(value, "12");
    }

    #[test]
    fn requires_room_alias_param() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch without a valid room alias")
            }
        }
        assert!(resolve_matrix(&HashMap::new(), &Unused).is_err());
        assert!(resolve_matrix(&params(""), &Unused).is_err());
        assert!(resolve_matrix(&params("no-colon-here"), &Unused).is_err());
    }

    #[test]
    fn rejects_path_breaking_server_fqdn_params() {
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never fetch with an invalid server_fqdn")
            }
        }
        let mut p = params("twim:matrix.org");
        p.insert("server_fqdn".to_string(), "evil.com/../etc".to_string());
        assert!(resolve_matrix(&p, &Unused).is_err());
    }

    #[test]
    fn errors_when_the_field_is_missing() {
        let fetcher = FakeFetcher {
            expected_url: "https://matrix.org/_matrix/client/unstable/im.nheko.summary/rooms/%23twim%3Amatrix.org/summary",
            body: r#"{}"#,
        };
        assert!(resolve_matrix(&params("twim:matrix.org"), &fetcher).is_err());
    }
}
