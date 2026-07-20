use std::fmt;

#[derive(Debug)]
pub enum FontError {
    UnexpectedEof,
    UnsupportedOutlineFormat,
    MissingTable(&'static str),
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontError::UnexpectedEof => write!(f, "unexpected end of font data"),
            FontError::UnsupportedOutlineFormat => write!(
                f,
                "unsupported outline format (only TrueType glyf outlines are supported, not CFF/PostScript)"
            ),
            FontError::MissingTable(tag) => write!(f, "missing required table: {tag}"),
        }
    }
}

impl std::error::Error for FontError {}
