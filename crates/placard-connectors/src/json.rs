use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub(crate) fn get(&self, path: &str) -> Option<&Value> {
        let mut current = self;
        for part in path.split('.') {
            let Value::Object(fields) = current else {
                return None;
            };
            current = &fields.iter().find(|(k, _)| k == part)?.1;
        }
        Some(current)
    }

    pub(crate) fn as_text(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(format_number(*n)),
            Value::Bool(b) => Some(b.to_string()),
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

const MAX_DEPTH: u32 = 128;

pub(crate) fn parse(input: &str) -> Result<Value, String> {
    let mut p = Parser {
        chars: input.chars().peekable(),
        depth: 0,
    };
    p.skip_whitespace();
    let value = p.parse_value()?;
    p.skip_whitespace();
    if p.chars.peek().is_some() {
        return Err("unexpected trailing data after JSON value".to_string());
    }
    Ok(value)
}

struct Parser<'a> {
    chars: Peekable<Chars<'a>>,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn skip_whitespace(&mut self) {
        while matches!(self.chars.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.chars.next();
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_whitespace();
        match self.chars.peek() {
            Some('{') => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err("maximum nesting depth exceeded".to_string());
                }
                let result = self.parse_object();
                self.depth -= 1;
                result
            }
            Some('[') => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err("maximum nesting depth exceeded".to_string());
                }
                let result = self.parse_array();
                self.depth -= 1;
                result
            }
            Some('"') => self.parse_string().map(Value::String),
            Some('t') => self.expect_literal("true", Value::Bool(true)),
            Some('f') => self.expect_literal("false", Value::Bool(false)),
            Some('n') => self.expect_literal("null", Value::Null),
            Some(c) if *c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(format!("unexpected character '{c}'")),
            None => Err("unexpected end of input".to_string()),
        }
    }

    fn expect_literal(&mut self, literal: &str, value: Value) -> Result<Value, String> {
        for expected in literal.chars() {
            match self.chars.next() {
                Some(c) if c == expected => {}
                _ => return Err(format!("expected literal '{literal}'")),
            }
        }
        Ok(value)
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        match self.chars.next() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(format!("expected '{expected}', found '{c}'")),
            None => Err(format!("expected '{expected}', found end of input")),
        }
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.expect_char('{')?;
        let mut fields = Vec::new();
        self.skip_whitespace();
        if self.chars.peek() == Some(&'}') {
            self.chars.next();
            return Ok(Value::Object(fields));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            let value = self.parse_value()?;
            match fields.iter_mut().find(|(k, _)| *k == key) {
                Some(existing) => existing.1 = value,
                None => fields.push((key, value)),
            }
            self.skip_whitespace();
            match self.chars.next() {
                Some(',') => continue,
                Some('}') => break,
                Some(c) => return Err(format!("expected ',' or '}}', found '{c}'")),
                None => return Err("unexpected end of input in object".to_string()),
            }
        }
        Ok(Value::Object(fields))
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.expect_char('[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.chars.peek() == Some(&']') {
            self.chars.next();
            return Ok(Value::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.chars.next() {
                Some(',') => continue,
                Some(']') => break,
                Some(c) => return Err(format!("expected ',' or ']', found '{c}'")),
                None => return Err("unexpected end of input in array".to_string()),
            }
        }
        Ok(Value::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_char('"')?;
        let mut out = String::new();
        loop {
            match self.chars.next() {
                Some('"') => break,
                Some('\\') => match self.chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let code = self.parse_hex4()?;
                        if (0xD800..=0xDBFF).contains(&code) {
                            if self.chars.peek() != Some(&'\\') {
                                return Err("unpaired UTF-16 surrogate in \\u escape".to_string());
                            }
                            self.chars.next();
                            if self.chars.next() != Some('u') {
                                return Err("unpaired UTF-16 surrogate in \\u escape".to_string());
                            }
                            let low = self.parse_hex4()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(
                                    "expected low surrogate after high surrogate in \\u escape"
                                        .to_string(),
                                );
                            }
                            let combined = 0x10000 + (code - 0xD800) * 0x400 + (low - 0xDC00);
                            out.push(
                                char::from_u32(combined)
                                    .ok_or_else(|| "invalid surrogate pair".to_string())?,
                            );
                        } else if (0xDC00..=0xDFFF).contains(&code) {
                            return Err("unpaired low surrogate in \\u escape".to_string());
                        } else {
                            out.push(char::from_u32(code).ok_or_else(|| {
                                format!("invalid unicode escape '\\u{code:04x}'")
                            })?);
                        }
                    }
                    Some(c) => return Err(format!("invalid escape '\\{c}'")),
                    None => return Err("unexpected end of input in string escape".to_string()),
                },
                Some(c) if (c as u32) < 0x20 => {
                    return Err("control character in string literal".to_string());
                }
                Some(c) => out.push(c),
                None => return Err("unexpected end of input in string".to_string()),
            }
        }
        Ok(out)
    }

    fn parse_hex4(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let c = self
                .chars
                .next()
                .ok_or("unexpected end of input in \\u escape")?;
            let digit = c
                .to_digit(16)
                .ok_or_else(|| format!("invalid hex digit '{c}'"))?;
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let mut text = String::new();
        if self.chars.peek() == Some(&'-') {
            text.push(self.chars.next().unwrap());
        }
        match self.chars.peek() {
            Some('0') => text.push(self.chars.next().unwrap()),
            Some(c) if c.is_ascii_digit() => {
                while matches!(self.chars.peek(), Some(c) if c.is_ascii_digit()) {
                    text.push(self.chars.next().unwrap());
                }
            }
            _ => return Err("invalid number: expected a digit".to_string()),
        }
        if self.chars.peek() == Some(&'.') {
            text.push(self.chars.next().unwrap());
            if !matches!(self.chars.peek(), Some(c) if c.is_ascii_digit()) {
                return Err("invalid number: expected a digit after '.'".to_string());
            }
            while matches!(self.chars.peek(), Some(c) if c.is_ascii_digit()) {
                text.push(self.chars.next().unwrap());
            }
        }
        if matches!(self.chars.peek(), Some('e' | 'E')) {
            text.push(self.chars.next().unwrap());
            if matches!(self.chars.peek(), Some('+' | '-')) {
                text.push(self.chars.next().unwrap());
            }
            if !matches!(self.chars.peek(), Some(c) if c.is_ascii_digit()) {
                return Err("invalid number: expected a digit in exponent".to_string());
            }
            while matches!(self.chars.peek(), Some(c) if c.is_ascii_digit()) {
                text.push(self.chars.next().unwrap());
            }
        }
        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| format!("invalid number literal '{text}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_flat_object() {
        let v = parse(r#"{"stargazers_count": 12483, "name": "placard"}"#).unwrap();
        assert_eq!(v.get("stargazers_count"), Some(&Value::Number(12483.0)));
        assert_eq!(v.get("name").unwrap().as_text().unwrap(), "placard");
    }

    #[test]
    fn parses_nested_dot_path() {
        let v = parse(r#"{"data": {"count": 42}}"#).unwrap();
        assert_eq!(v.get("data.count"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn formats_whole_numbers_without_a_decimal_point() {
        let v = parse(r#"{"n": 12483}"#).unwrap();
        assert_eq!(v.get("n").unwrap().as_text().unwrap(), "12483");
    }

    #[test]
    fn handles_escapes_and_unicode_in_strings() {
        let v = parse(r#"{"s": "line1\nline2\té"}"#).unwrap();
        assert_eq!(
            v.get("s").unwrap().as_text().unwrap(),
            "line1\nline2\t\u{e9}"
        );
    }

    #[test]
    fn parses_arrays_and_booleans_and_null() {
        let v = parse(r#"[1, true, false, null, "x"]"#).unwrap();
        assert_eq!(
            v,
            Value::Array(vec![
                Value::Number(1.0),
                Value::Bool(true),
                Value::Bool(false),
                Value::Null,
                Value::String("x".to_string()),
            ])
        );
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse(r#"{"a": 1} garbage"#).is_err());
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(parse("{").is_err());
        assert!(parse("").is_err());
        assert!(parse("{\"a\": }").is_err());
    }

    #[test]
    fn missing_path_returns_none() {
        let v = parse(r#"{"a": 1}"#).unwrap();
        assert_eq!(v.get("b"), None);
        assert_eq!(v.get("a.b"), None);
    }

    #[test]
    fn decodes_surrogate_pairs() {
        let v = parse(r#"{"s": "😀"}"#).unwrap();
        assert_eq!(v.get("s").unwrap().as_text().unwrap(), "\u{1F600}");
    }

    #[test]
    fn rejects_unpaired_surrogates() {
        assert!(parse(r#"{"s": "\ud83d"}"#).is_err());
        assert!(parse(r#"{"s": "\ud83dx"}"#).is_err());
        assert!(parse(r#"{"s": "\udc00"}"#).is_err());
    }

    #[test]
    fn rejects_leading_zeros() {
        assert!(parse("01").is_err());
        assert!(parse("-01").is_err());
        assert!(parse("0.5").is_ok());
        assert!(parse("0").is_ok());
    }

    #[test]
    fn rejects_unescaped_control_characters() {
        assert!(parse("\"a\nb\"").is_err());
        assert!(parse("\"a\tb\"").is_err());
    }

    #[test]
    fn last_duplicate_key_wins() {
        let v = parse(r#"{"a": 1, "a": 2}"#).unwrap();
        assert_eq!(v.get("a"), Some(&Value::Number(2.0)));
    }

    #[test]
    fn rejects_excessive_nesting() {
        let deep = "[".repeat(200) + &"]".repeat(200);
        assert!(parse(&deep).is_err());
        let ok = "[".repeat(50) + &"]".repeat(50);
        assert!(parse(&ok).is_ok());
    }

    fn assert_all_ok(cases: &[&str]) {
        for c in cases {
            assert!(parse(c).is_ok(), "expected Ok for {c:?}");
        }
    }

    fn assert_all_err(cases: &[&str]) {
        for c in cases {
            assert!(parse(c).is_err(), "expected Err for {c:?}");
        }
    }

    #[test]
    fn accepts_top_level_scalars() {
        assert_all_ok(&["42", "-17.5", "\"hello\"", "true", "false", "null", "0"]);
    }

    #[test]
    fn accepts_valid_number_forms() {
        assert_all_ok(&[
            "0",
            "-0",
            "1",
            "-1",
            "42",
            "1.5",
            "-1.5",
            "0.5",
            "1.0",
            "1e10",
            "1E10",
            "1e+10",
            "1e-10",
            "1.5e10",
            "1.5E-10",
            "123456789",
            "-123456789",
            "0.0",
            "3.14159",
            "1e0",
            "9007199254740991",
            "-9007199254740991",
            "1.7976931348623157e308",
            "0e0",
            "10",
        ]);
    }

    #[test]
    fn rejects_invalid_number_forms() {
        assert_all_err(&[
            "01",
            "-01",
            "00",
            "1.",
            "-1.",
            "1.e5",
            ".5",
            "-.5",
            "1e",
            "1e+",
            "1e-",
            "+1",
            "1,0",
            "NaN",
            "Infinity",
            "-Infinity",
            "0x1",
            "1.2.3",
            "--1",
            "1-2",
            "",
            "-",
            ".",
        ]);
    }

    #[test]
    fn accepts_valid_string_escapes() {
        assert_eq!(parse(r#""""#).unwrap(), Value::String(String::new()));
        assert_eq!(
            parse(r#""hello""#).unwrap(),
            Value::String("hello".to_string())
        );
        assert_eq!(
            parse(r#""line1\nline2""#).unwrap(),
            Value::String("line1\nline2".to_string())
        );
        assert_eq!(
            parse(r#""a\tb""#).unwrap(),
            Value::String("a\tb".to_string())
        );
        assert_eq!(
            parse(r#""a\rb""#).unwrap(),
            Value::String("a\rb".to_string())
        );
        assert_eq!(
            parse(r#""quote\"quote""#).unwrap(),
            Value::String("quote\"quote".to_string())
        );
        assert_eq!(
            parse(r#""back\\slash""#).unwrap(),
            Value::String("back\\slash".to_string())
        );
        assert_eq!(
            parse(r#""fwd\/slash""#).unwrap(),
            Value::String("fwd/slash".to_string())
        );
        assert_eq!(
            parse(r#""bs\b""#).unwrap(),
            Value::String("bs\u{8}".to_string())
        );
        assert_eq!(
            parse(r#""ff\f""#).unwrap(),
            Value::String("ff\u{c}".to_string())
        );
        assert_eq!(
            parse(r#""\u0041""#).unwrap(),
            Value::String("A".to_string())
        );
        assert_eq!(
            parse(r#""\u00e9""#).unwrap(),
            Value::String("\u{e9}".to_string())
        );
        assert_eq!(
            parse(r#""\u0000""#).unwrap(),
            Value::String("\u{0}".to_string())
        );
        assert_eq!(
            parse(r#""😀""#).unwrap(),
            Value::String("\u{1F600}".to_string())
        );
    }

    #[test]
    fn rejects_invalid_string_forms() {
        assert_all_err(&[
            "\"unterminated",
            r#""bad escape \x""#,
            "\"trailing backslash\\\"",
            r#""\u12""#,
            r#""\u12zz""#,
            r#""\ud800\ud800""#,
            r#""\ude00""#,
            r#""\ud83d""#,
            r#""\ud83dnotalowsurrogate""#,
        ]);
    }

    #[test]
    fn rejects_raw_control_characters_in_strings() {
        assert!(parse("\"a\nb\"").is_err());
        assert!(parse("\"a\tb\"").is_err());
        assert!(parse("\"a\rb\"").is_err());
        assert!(parse("\"\u{0}\"").is_err());
        assert!(parse("\"\u{1f}\"").is_err());
    }

    #[test]
    fn accepts_valid_array_forms() {
        assert_all_ok(&[
            "[]",
            "[1]",
            "[1,2,3]",
            "[[1,2],[3,4]]",
            "[ 1 , 2 , 3 ]",
            "[1, \"two\", 3.0, true, false, null, [4]]",
        ]);
    }

    #[test]
    fn rejects_invalid_array_forms() {
        assert_all_err(&["[1,]", "[,1]", "[1 2]", "[1,,2]", "[", "]", "[1", "[1,"]);
    }

    #[test]
    fn accepts_valid_object_forms() {
        assert_all_ok(&[
            "{}",
            r#"{"a":1}"#,
            r#"{ "a" : 1 , "b" : 2 }"#,
            r#"{"a":{"b":{"c":1}}}"#,
            r#"{"a":[1,2,3]}"#,
        ]);
    }

    #[test]
    fn rejects_invalid_object_forms() {
        assert_all_err(&[
            r#"{"a":1,}"#,
            r#"{,"a":1}"#,
            r#"{"a" 1}"#,
            "{a:1}",
            "{'a':1}",
            r#"{1:"a"}"#,
            "{",
            "}",
            r#"{"a":1"#,
            r#"{"a":1,"#,
        ]);
    }

    #[test]
    fn ignores_only_spec_whitespace_between_tokens() {
        assert_all_ok(&[
            "  1  ",
            "\t1\t",
            "\n1\n",
            "\r1\r",
            " \t\n\r1 \t\n\r",
            "{ \"a\" : 1 }",
            "[ 1 , 2 ]",
        ]);
        assert_all_err(&["\u{b}1", "\u{c}1", "1\u{b}"]);
    }

    #[test]
    fn duplicate_keys_collapse_to_last_value_at_first_position() {
        let v = parse(r#"{"a":1,"b":2,"a":3}"#).unwrap();
        match v {
            Value::Object(fields) => {
                assert_eq!(
                    fields,
                    vec![
                        ("a".to_string(), Value::Number(3.0)),
                        ("b".to_string(), Value::Number(2.0)),
                    ]
                );
            }
            other => panic!("expected object, got {other:?}"),
        }
    }

    #[test]
    fn depth_limit_is_exact() {
        let at_limit = "[".repeat(128) + &"]".repeat(128);
        assert!(parse(&at_limit).is_ok());
        let over_limit = "[".repeat(129) + &"]".repeat(129);
        assert!(parse(&over_limit).is_err());
    }

    #[test]
    fn get_returns_none_through_non_object_shapes() {
        let v = parse(r#"{"a":[1,2,3]}"#).unwrap();
        assert_eq!(v.get("a.0"), None);
        let scalar = parse("42").unwrap();
        assert_eq!(scalar.get("anything"), None);
        assert_eq!(scalar.get(""), None);
    }

    #[test]
    fn as_text_covers_every_variant() {
        assert_eq!(Value::Null.as_text(), None);
        assert_eq!(Value::Bool(true).as_text().unwrap(), "true");
        assert_eq!(Value::Bool(false).as_text().unwrap(), "false");
        assert_eq!(Value::Number(0.0).as_text().unwrap(), "0");
        assert_eq!(Value::Number(-0.0).as_text().unwrap(), "0");
        assert_eq!(Value::Number(1.5).as_text().unwrap(), "1.5");
        assert_eq!(Value::Number(-1.5).as_text().unwrap(), "-1.5");
        assert_eq!(Value::Number(1e15).as_text().unwrap(), "1000000000000000");
        assert_eq!(Value::String("x".to_string()).as_text().unwrap(), "x");
        assert_eq!(Value::Array(vec![]).as_text(), None);
        assert_eq!(Value::Object(vec![]).as_text(), None);
    }

    #[test]
    fn parses_a_realistic_nested_api_payload() {
        let payload = r#"{
            "id": 42,
            "name": "placard",
            "stargazers_count": 12483,
            "archived": false,
            "homepage": null,
            "owner": {
                "login": "octocat",
                "id": 1
            },
            "topics": ["rust", "cli", "svg"],
            "license": {
                "key": "mit",
                "name": "MIT License"
            }
        }"#;
        let v = parse(payload).unwrap();
        assert_eq!(v.get("id").unwrap().as_text().unwrap(), "42");
        assert_eq!(v.get("owner.login").unwrap().as_text().unwrap(), "octocat");
        assert_eq!(v.get("license.key").unwrap().as_text().unwrap(), "mit");
        assert_eq!(v.get("archived").unwrap().as_text().unwrap(), "false");
        assert_eq!(v.get("homepage"), Some(&Value::Null));
        assert_eq!(
            v.get("topics"),
            Some(&Value::Array(vec![
                Value::String("rust".to_string()),
                Value::String("cli".to_string()),
                Value::String("svg".to_string()),
            ]))
        );
    }

    #[test]
    fn accepts_overflow_and_underflow_numbers_as_infinities_and_zero() {
        assert_eq!(parse("1e309").unwrap(), Value::Number(f64::INFINITY));
        assert_eq!(parse("-1e309").unwrap(), Value::Number(f64::NEG_INFINITY));
        assert_eq!(parse("1e-400").unwrap(), Value::Number(0.0));
    }

    #[test]
    fn accepts_numbers_that_lose_precision_in_f64() {
        let v = parse("123456789012345678901234567890").unwrap();
        match v {
            Value::Number(n) => assert!(n.is_finite() && n > 0.0),
            other => panic!("expected number, got {other:?}"),
        }
    }

    #[test]
    fn accepts_every_sign_and_exponent_combination() {
        assert_all_ok(&[
            "0e1", "0e+1", "0e-1", "-0e0", "-0.0", "5e-324", "1e100", "1e-100", "-1.5e+2", "3.0e0",
        ]);
    }

    #[test]
    fn combines_full_range_of_surrogate_pair_escapes() {
        let min_astral = parse(r#""\ud800\udc00""#).unwrap();
        assert_eq!(
            min_astral,
            Value::String(char::from_u32(0x10000).unwrap().to_string())
        );
        let max_astral = parse(r#""\udbff\udfff""#).unwrap();
        assert_eq!(
            max_astral,
            Value::String(char::from_u32(0x10FFFF).unwrap().to_string())
        );
        let emoji_via_escape = parse(r#""\ud83d\ude00""#).unwrap();
        assert_eq!(emoji_via_escape, Value::String("\u{1F600}".to_string()));
        let emoji_via_literal_bytes = parse("\"\u{1F600}\"").unwrap();
        assert_eq!(emoji_via_escape, emoji_via_literal_bytes);
    }

    #[test]
    fn allows_del_and_raw_multibyte_utf8_unescaped_in_strings() {
        assert_eq!(
            parse("\"\u{7f}\"").unwrap(),
            Value::String("\u{7f}".to_string())
        );
        assert_eq!(
            parse("\"\u{65e5}\u{672c}\u{8a9e}\"").unwrap(),
            Value::String("\u{65e5}\u{672c}\u{8a9e}".to_string())
        );
        assert_eq!(
            parse("\"\u{1f600}\u{1f601}\"").unwrap(),
            Value::String("\u{1f600}\u{1f601}".to_string())
        );
    }

    #[test]
    fn allows_escapes_in_object_keys() {
        let v = parse(r#"{"a\tb": 1, "line\nbreak": 2}"#).unwrap();
        assert_eq!(v.get("a\tb"), Some(&Value::Number(1.0)));
        assert_eq!(v.get("line\nbreak"), Some(&Value::Number(2.0)));
    }

    #[test]
    fn deeply_nested_objects_respect_depth_limit() {
        let at_limit = "{\"a\":".repeat(128) + "1" + &"}".repeat(128);
        assert!(parse(&at_limit).is_ok());
        let over_limit = "{\"a\":".repeat(129) + "1" + &"}".repeat(129);
        assert!(parse(&over_limit).is_err());
    }

    #[test]
    fn mixed_array_object_nesting_respects_depth_limit() {
        let at_limit = "[{\"a\":".repeat(64) + "1" + &"}]".repeat(64);
        assert!(parse(&at_limit).is_ok());
        let over_limit = "[{\"a\":".repeat(65) + "1" + &"}]".repeat(65);
        assert!(parse(&over_limit).is_err());
    }

    #[test]
    fn rejects_byte_order_mark_prefix() {
        assert!(parse("\u{feff}{}").is_err());
    }

    #[test]
    fn rejects_json5_and_comment_extensions() {
        assert_all_err(&[
            "1 // comment",
            "/* comment */ 1",
            "{a: 1}",
            "{\"a\": 1,}",
            "[1, 2,]",
            "undefined",
            "0b101",
            "0o17",
        ]);
    }

    #[test]
    fn accepts_whitespace_only_empty_containers() {
        assert_all_ok(&["[   ]", "{   }", "[\t\n\r]", "{\t\n\r}"]);
    }

    #[test]
    fn rejects_whitespace_only_input() {
        assert!(parse("   ").is_err());
        assert!(parse("\t\n\r").is_err());
    }

    #[test]
    fn format_number_handles_negative_and_boundary_values() {
        assert_eq!(Value::Number(-42.0).as_text().unwrap(), "-42");
        assert_eq!(
            Value::Number(999_999_999_999_999.0).as_text().unwrap(),
            "999999999999999"
        );
        assert_eq!(Value::Number(1e15).as_text().unwrap(), "1000000000000000");
        assert_eq!(Value::Number(-1e15).as_text().unwrap(), "-1000000000000000");
        assert_eq!(Value::Number(f64::INFINITY).as_text().unwrap(), "inf");
        assert_eq!(Value::Number(f64::NEG_INFINITY).as_text().unwrap(), "-inf");
    }

    #[test]
    fn object_key_containing_a_literal_dot_is_not_reachable_via_dotted_get() {
        let v = parse(r#"{"a.b": 1}"#).unwrap();
        assert_eq!(v.get("a.b"), None);
        match v {
            Value::Object(fields) => {
                assert_eq!(fields, vec![("a.b".to_string(), Value::Number(1.0))]);
            }
            other => panic!("expected object, got {other:?}"),
        }
    }

    #[test]
    fn parses_a_kitchen_sink_document() {
        let payload = r#"{
            "string": "hello \"world\"\n\té😀",
            "int": 42,
            "negative": -17,
            "float": 3.14159,
            "exponent": 1.5e10,
            "zero": 0,
            "true": true,
            "false": false,
            "null": null,
            "empty_array": [],
            "empty_object": {},
            "nested_array": [1, [2, [3, [4, [5]]]]],
            "array_of_objects": [
                {"id": 1, "tags": ["a", "b"]},
                {"id": 2, "tags": []}
            ],
            "deep": {"a": {"b": {"c": {"d": "leaf"}}}},
            "duplicate": "first",
            "duplicate": "second"
        }"#;
        let v = parse(payload).unwrap();
        assert_eq!(
            v.get("string").unwrap().as_text().unwrap(),
            "hello \"world\"\n\t\u{e9}\u{1f600}"
        );
        assert_eq!(v.get("int").unwrap(), &Value::Number(42.0));
        assert_eq!(v.get("negative").unwrap(), &Value::Number(-17.0));
        assert_eq!(v.get("float").unwrap(), &Value::Number(3.14159));
        assert_eq!(v.get("exponent").unwrap(), &Value::Number(1.5e10));
        assert_eq!(v.get("zero").unwrap(), &Value::Number(0.0));
        assert_eq!(v.get("true").unwrap(), &Value::Bool(true));
        assert_eq!(v.get("false").unwrap(), &Value::Bool(false));
        assert_eq!(v.get("null").unwrap(), &Value::Null);
        assert_eq!(v.get("empty_array").unwrap(), &Value::Array(vec![]));
        assert_eq!(v.get("empty_object").unwrap(), &Value::Object(vec![]));
        assert_eq!(v.get("deep.a.b.c.d").unwrap().as_text().unwrap(), "leaf");
        assert_eq!(v.get("duplicate").unwrap().as_text().unwrap(), "second");
        match v.get("array_of_objects").unwrap() {
            Value::Array(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].get("id").unwrap(), &Value::Number(1.0));
                assert_eq!(
                    items[0].get("tags").unwrap(),
                    &Value::Array(vec![
                        Value::String("a".to_string()),
                        Value::String("b".to_string())
                    ])
                );
                assert_eq!(items[1].get("tags").unwrap(), &Value::Array(vec![]));
            }
            other => panic!("expected array, got {other:?}"),
        }
        match v.get("nested_array").unwrap() {
            Value::Array(items) => assert_eq!(items[0], Value::Number(1.0)),
            other => panic!("expected array, got {other:?}"),
        }
    }
}
