use crate::colors::named_color;
use crate::types::{
    AttrMatch, Color, Combinator, Declaration, Diagnostic, Rule, Selector, SimpleSelector,
    Stylesheet, Value,
};

const MAX_SNIPPET_LEN: usize = 80;

fn truncate_snippet(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() > MAX_SNIPPET_LEN {
        let truncated: String = s.chars().take(MAX_SNIPPET_LEN).collect();
        format!("{truncated}\u{2026}")
    } else {
        s.to_string()
    }
}

pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_str(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) -> bool {
        let mut skipped_ws = false;
        loop {
            if self.peek_char().is_some_and(|c| c.is_whitespace()) {
                skipped_ws = true;
                self.advance();
                continue;
            }
            if self.peek_str("/*") {
                self.pos += 2;
                while !self.eof() && !self.peek_str("*/") {
                    self.advance();
                }
                if self.peek_str("*/") {
                    self.pos += 2;
                }
                continue;
            }
            break;
        }
        skipped_ws
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                s.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        s
    }

    fn read_number(&mut self) -> Option<f32> {
        let start = self.pos;
        if self.peek_char() == Some('-') {
            self.advance();
        }
        let mut has_digits = false;
        while self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
            has_digits = true;
            self.advance();
        }
        if self.peek_char() == Some('.') {
            self.advance();
            while self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                has_digits = true;
                self.advance();
            }
        }
        if !has_digits {
            self.pos = start;
            return None;
        }
        self.input[start..self.pos].parse().ok()
    }

    pub fn parse_stylesheet(&mut self) -> Stylesheet {
        let mut rules = Vec::new();
        let mut skip_start: Option<usize> = None;
        loop {
            self.skip_whitespace_and_comments();
            if self.eof() {
                break;
            }
            let pos_before = self.pos;
            if let Some(rule) = self.parse_rule() {
                if let Some(start) = skip_start.take() {
                    self.push_skip_diagnostic(start, pos_before, "rule");
                }
                rules.push(rule);
                continue;
            }
            if skip_start.is_none() {
                skip_start = Some(pos_before);
            }
            if self.pos == pos_before {
                self.advance();
            }
        }
        if let Some(start) = skip_start {
            let end = self.pos;
            self.push_skip_diagnostic(start, end, "rule");
        }
        Stylesheet { rules }
    }

    fn push_skip_diagnostic(&mut self, start: usize, end: usize, kind: &str) {
        let snippet = truncate_snippet(&self.input[start..end]);
        if !snippet.is_empty() {
            self.diagnostics.push(Diagnostic::warning(format!(
                "couldn't parse CSS {kind}, skipped: `{snippet}`"
            )));
        }
    }

    fn parse_rule(&mut self) -> Option<Rule> {
        let selectors = self.parse_selector_list()?;
        self.skip_whitespace_and_comments();
        if self.peek_char() != Some('{') {
            return None;
        }
        self.advance();
        let declarations = self.parse_declarations();
        self.skip_whitespace_and_comments();
        if self.peek_char() == Some('}') {
            self.advance();
        }
        Some(Rule {
            selectors,
            declarations,
        })
    }

    fn parse_selector_list(&mut self) -> Option<Vec<Selector>> {
        let mut selectors = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let sel = self.parse_selector()?;
            selectors.push(sel);
            self.skip_whitespace_and_comments();
            if self.peek_char() == Some(',') {
                self.advance();
                continue;
            }
            break;
        }
        if selectors.is_empty() {
            None
        } else {
            Some(selectors)
        }
    }

    fn parse_selector(&mut self) -> Option<Selector> {
        let mut parts = vec![self.parse_simple_selector()?];
        let mut combinators = Vec::new();

        loop {
            let had_ws = self.skip_whitespace_and_comments();
            match self.peek_char() {
                Some('>') => {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    let next = self.parse_simple_selector()?;
                    combinators.push(Combinator::Child);
                    parts.push(next);
                }
                Some('+') => {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    let next = self.parse_simple_selector()?;
                    combinators.push(Combinator::Adjacent);
                    parts.push(next);
                }
                Some('~') => {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    let next = self.parse_simple_selector()?;
                    combinators.push(Combinator::General);
                    parts.push(next);
                }
                Some('{') | Some(',') | None => break,
                Some(_) if had_ws => match self.parse_simple_selector() {
                    Some(next) => {
                        combinators.push(Combinator::Descendant);
                        parts.push(next);
                    }
                    None => break,
                },
                Some(_) => break,
            }
        }

        Some(Selector { parts, combinators })
    }

    fn parse_simple_selector(&mut self) -> Option<SimpleSelector> {
        let mut sel = SimpleSelector::default();

        if self.peek_char().is_some_and(is_ident_start) {
            sel.tag = Some(self.read_ident());
        }

        loop {
            match self.peek_char() {
                Some('.') => {
                    self.advance();
                    let class = self.read_ident();
                    if !class.is_empty() {
                        sel.classes.push(class);
                    }
                }
                Some('#') => {
                    self.advance();
                    let id = self.read_ident();
                    if !id.is_empty() {
                        sel.id = Some(id);
                    }
                }
                Some('[') => {
                    self.advance();
                    if let Some(attr) = self.parse_attr_selector() {
                        sel.attrs.push(attr);
                    }
                }
                _ => break,
            }
        }

        if sel.is_empty() { None } else { Some(sel) }
    }

    fn parse_attr_selector(&mut self) -> Option<(String, AttrMatch)> {
        self.skip_whitespace_and_comments();
        let name = self.read_ident();
        if name.is_empty() {
            self.skip_to_bracket_close();
            return None;
        }
        self.skip_whitespace_and_comments();

        if self.peek_char() != Some('=') {
            self.skip_to_bracket_close();
            return Some((name, AttrMatch::Present));
        }
        self.advance();
        self.skip_whitespace_and_comments();

        let value = if self.peek_char() == Some('"') || self.peek_char() == Some('\'') {
            let quote = self.advance().unwrap();
            let mut s = String::new();
            while let Some(c) = self.peek_char() {
                if c == quote {
                    break;
                }
                s.push(c);
                self.advance();
            }
            s
        } else {
            self.read_ident()
        };

        self.skip_to_bracket_close();
        Some((name, AttrMatch::Equals(value)))
    }

    fn skip_to_bracket_close(&mut self) {
        self.skip_whitespace_and_comments();
        while let Some(c) = self.peek_char() {
            if c == ']' {
                self.advance();
                break;
            }
            self.advance();
        }
    }

    pub(crate) fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut decls = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            match self.peek_char() {
                None | Some('}') => break,
                Some(';') => {
                    self.advance();
                }
                _ => {
                    let start = self.pos;
                    if let Some(decl) = self.parse_declaration() {
                        decls.push(decl);
                    } else {
                        while !self.eof()
                            && self.peek_char() != Some(';')
                            && self.peek_char() != Some('}')
                        {
                            self.advance();
                        }
                        let end = self.pos;
                        if self.peek_char() == Some(';') {
                            self.advance();
                        }
                        self.push_skip_diagnostic(start, end, "declaration");
                    }
                }
            }
        }
        decls
    }

    fn parse_declaration(&mut self) -> Option<Declaration> {
        let property = self.read_ident();
        if property.is_empty() {
            return None;
        }
        let property = property.to_ascii_lowercase();
        self.skip_whitespace_and_comments();
        if self.peek_char() != Some(':') {
            return None;
        }
        self.advance();
        self.skip_whitespace_and_comments();

        let preserve_case = property == "font-family" || property == "font";
        let value = self.parse_value_list(preserve_case)?;
        self.skip_whitespace_and_comments();
        if self.peek_char() == Some(';') {
            self.advance();
        }
        Some(Declaration { property, value })
    }

    fn parse_value_list(&mut self, preserve_case: bool) -> Option<Value> {
        let mut values = vec![self.parse_value(preserve_case)?];
        loop {
            let had_ws = self.skip_whitespace_and_comments();
            let had_separator = match self.peek_char() {
                Some('/') | Some(',') => {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    true
                }
                _ => false,
            };
            if !had_ws && !had_separator {
                break;
            }
            match self.peek_char() {
                Some(';') | Some('}') | None => break,
                _ => match self.parse_value(preserve_case) {
                    Some(v) => values.push(v),
                    None => break,
                },
            }
        }
        if values.len() == 1 {
            values.pop()
        } else {
            Some(Value::List(values))
        }
    }

    fn parse_value(&mut self, preserve_case: bool) -> Option<Value> {
        match self.peek_char()? {
            '#' => self.parse_hex_color(),
            '"' | '\'' => self.parse_quoted_string(preserve_case),
            c if c.is_ascii_digit() || c == '-' || c == '.' => self.parse_number_value(),
            c if c.is_ascii_alphabetic() => self.parse_ident_value(preserve_case),
            _ => None,
        }
    }

    fn parse_quoted_string(&mut self, preserve_case: bool) -> Option<Value> {
        let quote = self.advance()?;
        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c == quote {
                break;
            }
            s.push(c);
            self.advance();
        }
        if self.peek_char() == Some(quote) {
            self.advance();
        }
        if !preserve_case {
            s = s.to_ascii_lowercase();
        }
        Some(Value::Keyword(s))
    }

    fn parse_hex_color(&mut self) -> Option<Value> {
        self.advance();
        let mut hex = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_ascii_hexdigit() {
                hex.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        let color = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Color::rgb(r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Color::rgb(r, g, b)
            }
            _ => return None,
        };
        Some(Value::Color(color))
    }

    fn parse_number_value(&mut self) -> Option<Value> {
        let number = self.read_number()?;
        if self.peek_str("px") {
            self.pos += 2;
            Some(Value::Length(number))
        } else if self.peek_str("rem") {
            self.pos += 3;
            Some(Value::Rem(number))
        } else if self.peek_str("em") {
            self.pos += 2;
            Some(Value::Em(number))
        } else if self.peek_str("%") {
            self.pos += 1;
            Some(Value::Percent(number))
        } else if self.peek_str("fr") {
            self.pos += 2;
            Some(Value::Fr(number))
        } else {
            Some(Value::Length(number))
        }
    }

    fn parse_ident_value(&mut self, preserve_case: bool) -> Option<Value> {
        let ident = self.read_ident();
        if ident.is_empty() {
            return None;
        }
        let lower = ident.to_ascii_lowercase();
        if (lower == "rgb" || lower == "rgba") && self.peek_char() == Some('(') {
            return self.parse_rgb_function();
        }
        if let Some(color) = named_color(&lower) {
            return Some(Value::Color(color));
        }
        Some(Value::Keyword(if preserve_case { ident } else { lower }))
    }

    fn parse_rgb_function(&mut self) -> Option<Value> {
        self.advance();
        let mut components = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            components.push(self.read_number()?);
            self.skip_whitespace_and_comments();
            match self.peek_char() {
                Some(',') => {
                    self.advance();
                }
                Some(')') => {
                    self.advance();
                    break;
                }
                _ => return None,
            }
        }
        let r = *components.first()? as u8;
        let g = *components.get(1)? as u8;
        let b = *components.get(2)? as u8;
        let a = components
            .get(3)
            .map(|a| (*a * 255.0).round() as u8)
            .unwrap_or(255);
        Some(Value::Color(Color::rgba(r, g, b, a)))
    }
}
