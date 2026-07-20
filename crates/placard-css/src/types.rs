#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32),
    Percent(f32),
    Em(f32),
    Rem(f32),
    Fr(f32),
    Color(Color),
    List(Vec<Value>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
    Adjacent,
    General,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttrMatch {
    Present,
    Equals(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimpleSelector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, AttrMatch)>,
}

impl SimpleSelector {
    pub fn is_empty(&self) -> bool {
        self.tag.is_none() && self.id.is_none() && self.classes.is_empty() && self.attrs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub parts: Vec<SimpleSelector>,
    pub combinators: Vec<Combinator>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Keyword(k) => write!(f, "{k}"),
            Value::Length(n) => write!(f, "{n}px"),
            Value::Percent(n) => write!(f, "{n}%"),
            Value::Em(n) => write!(f, "{n}em"),
            Value::Rem(n) => write!(f, "{n}rem"),
            Value::Fr(n) => write!(f, "{n}fr"),
            Value::Color(c) => write!(f, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
            Value::List(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{item}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::fmt::Display for SimpleSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(tag) = &self.tag {
            write!(f, "{tag}")?;
        }
        if let Some(id) = &self.id {
            write!(f, "#{id}")?;
        }
        for class in &self.classes {
            write!(f, ".{class}")?;
        }
        for (name, matcher) in &self.attrs {
            match matcher {
                AttrMatch::Present => write!(f, "[{name}]")?,
                AttrMatch::Equals(v) => write!(f, "[{name}=\"{v}\"]")?,
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.parts[0])?;
        for (part, combinator) in self.parts[1..].iter().zip(&self.combinators) {
            let sep = match combinator {
                Combinator::Descendant => " ",
                Combinator::Child => " > ",
                Combinator::Adjacent => " + ",
                Combinator::General => " ~ ",
            };
            write!(f, "{sep}{part}")?;
        }
        Ok(())
    }
}
