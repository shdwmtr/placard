use crate::Fetcher;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    pub required: bool,
    pub example: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct PresetMeta {
    pub preset: &'static str,
    pub service: &'static str,
    pub description: &'static str,
    pub params: &'static [Param],
    /// Whether this preset's resolved value is a plain number (safe to
    /// reformat with `data-number-format`) as opposed to text -- a version
    /// string, a license name, a status word, a date, or anything else that
    /// isn't a bare number. Not inferred at runtime: `data-number-format`
    /// still works on any preset regardless of this flag (it's a no-op if
    /// the resolved value doesn't parse as a number), but this is what lets
    /// the docs only *advertise* the feature where it actually applies.
    pub numeric: bool,
    pub resolve: fn(&HashMap<String, String>, &dyn Fetcher) -> Result<String, String>,
}
