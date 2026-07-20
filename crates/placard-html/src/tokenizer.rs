const ENTITIES: &[(&str, char)] = &[
    ("amp;", '&'),
    ("lt;", '<'),
    ("gt;", '>'),
    ("quot;", '"'),
    ("apos;", '\''),
    ("nbsp;", '\u{00A0}'),
];

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    StartTag {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Text(String),
}

pub struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
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

    fn skip_whitespace(&mut self) {
        while self.peek_char().is_some_and(|c| c.is_whitespace()) {
            self.advance();
        }
    }

    fn skip_until(&mut self, target: char) {
        while let Some(c) = self.peek_char() {
            if c == target {
                break;
            }
            self.advance();
        }
    }

    fn decode_entity_at(&self, start: usize) -> Option<(char, usize)> {
        let rest = self.input.get(start..)?;

        if let Some(after_hash) = rest.strip_prefix('#') {
            let (digits, radix) = match after_hash
                .strip_prefix('x')
                .or_else(|| after_hash.strip_prefix('X'))
            {
                Some(hex) => (hex, 16),
                None => (after_hash, 10),
            };
            let digit_len = digits
                .find(|c: char| !c.is_digit(radix))
                .unwrap_or(digits.len());
            if digit_len > 0 && digits[digit_len..].starts_with(';') {
                let prefix_len = rest.len() - digits.len();
                if let Ok(code) = u32::from_str_radix(&digits[..digit_len], radix) {
                    if let Some(ch) = char::from_u32(code) {
                        return Some((ch, prefix_len + digit_len + 1));
                    }
                }
            }
        }

        for (name, ch) in ENTITIES {
            if rest.starts_with(name) {
                return Some((*ch, name.len()));
            }
        }
        None
    }

    fn read_tag_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_ascii_alphanumeric() || c == '-' {
                name.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        name
    }

    fn read_attribute(&mut self) -> Option<(String, String)> {
        let mut name = String::new();
        while let Some(c) = self.peek_char() {
            if c == '=' || c == '>' || c == '/' || c.is_whitespace() {
                break;
            }
            name.push(c);
            self.pos += c.len_utf8();
        }
        if name.is_empty() {
            self.advance();
            return None;
        }

        self.skip_whitespace();
        let mut value = String::new();
        if self.peek_char() == Some('=') {
            self.advance();
            self.skip_whitespace();
            match self.peek_char() {
                Some(q @ ('"' | '\'')) => {
                    self.advance();
                    while let Some(c) = self.peek_char() {
                        if c == q {
                            self.advance();
                            break;
                        }
                        if c == '&' {
                            if let Some((decoded, len)) = self.decode_entity_at(self.pos + 1) {
                                value.push(decoded);
                                self.pos += 1 + len;
                                continue;
                            }
                        }
                        value.push(c);
                        self.pos += c.len_utf8();
                    }
                }
                _ => {
                    while let Some(c) = self.peek_char() {
                        if c.is_whitespace() || c == '>' {
                            break;
                        }
                        value.push(c);
                        self.pos += c.len_utf8();
                    }
                }
            }
        }

        Some((name.to_ascii_lowercase(), value))
    }

    fn read_start_tag(&mut self) -> Token {
        self.advance();
        let name = self.read_tag_name().to_ascii_lowercase();
        let mut attrs = Vec::new();
        let mut self_closing = false;

        loop {
            self.skip_whitespace();
            match self.peek_char() {
                None => break,
                Some('>') => {
                    self.advance();
                    break;
                }
                Some('/') => {
                    self.advance();
                    self.skip_whitespace();
                    if self.peek_char() == Some('>') {
                        self.advance();
                        self_closing = true;
                        break;
                    }
                }
                Some(_) => {
                    if let Some(attr) = self.read_attribute() {
                        attrs.push(attr);
                    }
                }
            }
        }

        Token::StartTag {
            name,
            attrs,
            self_closing,
        }
    }

    fn read_end_tag(&mut self) -> Token {
        self.pos += 2;
        let name = self.read_tag_name().to_ascii_lowercase();
        self.skip_until('>');
        if self.peek_char() == Some('>') {
            self.advance();
        }
        Token::EndTag { name }
    }

    fn read_text(&mut self) -> Token {
        let mut text = String::new();
        while let Some(c) = self.peek_char() {
            if c == '<' {
                break;
            }
            if c == '&' {
                if let Some((decoded, len)) = self.decode_entity_at(self.pos + 1) {
                    text.push(decoded);
                    self.pos += 1 + len;
                    continue;
                }
            }
            text.push(c);
            self.pos += c.len_utf8();
        }
        Token::Text(text)
    }

    pub fn read_raw_text_until(&mut self, tag_name: &str) -> String {
        let mut text = String::new();
        loop {
            if self.eof() {
                break;
            }
            if self.peek_char() == Some('<') {
                let rest = &self.input[self.pos..];
                if let Some(after_slash) = rest.strip_prefix("</") {
                    if after_slash.len() >= tag_name.len()
                        && after_slash[..tag_name.len()].eq_ignore_ascii_case(tag_name)
                        && after_slash[tag_name.len()..]
                            .chars()
                            .next()
                            .is_some_and(|c| c == '>' || c.is_whitespace())
                    {
                        break;
                    }
                }
            }
            if let Some(c) = self.advance() {
                text.push(c);
            }
        }
        text
    }

    pub fn next_token(&mut self) -> Option<Token> {
        loop {
            if self.eof() {
                return None;
            }
            if self.peek_char() != Some('<') {
                return Some(self.read_text());
            }
            if self.peek_str("<!--") {
                self.pos += 4;
                while !self.eof() && !self.peek_str("-->") {
                    self.advance();
                }
                if self.peek_str("-->") {
                    self.pos += 3;
                }
                continue;
            }
            if self.peek_str("<!") {
                self.skip_until('>');
                if self.peek_char() == Some('>') {
                    self.advance();
                }
                continue;
            }
            if self.peek_str("</") {
                return Some(self.read_end_tag());
            }
            let next_after_lt = self.input[self.pos + 1..].chars().next();
            if next_after_lt.is_some_and(|c| c.is_ascii_alphabetic()) {
                return Some(self.read_start_tag());
            }
            self.advance();
            return Some(Token::Text("<".to_string()));
        }
    }
}
