//! `data-number-format` support: a tiny, bounded, printf-inspired grammar
//! for reformatting a resolved preset's value when it happens to be a plain
//! number -- this is not a general expression evaluator, there's no code
//! execution and no open-ended computation, just a fixed lookup table for
//! the `K`/`M`/`B`/`T` scaling suffixes.
//!
//! Grammar: `%[,][.N](f|d|i)[K|M|B|T]`
//!   - `,`        -- group the output with thousands separators
//!   - `.N`       -- N decimal places (meaningful with `f` only; defaults
//!                   to 2 if omitted)
//!   - `f`        -- fixed-point
//!   - `d` or `i` -- rounded to the nearest integer
//!   - `K`/`M`/`B`/`T` -- divides the value by 1e3/1e6/1e9/1e12 *before*
//!                   formatting, then appends that same letter to the
//!                   output (`1000000` + `%.0fK` -> `"1000K"`)
//!
//! Anything that doesn't fit -- a malformed spec, or a value that isn't a
//! plain number to begin with -- returns `None`, and the caller falls back
//! to the original text unchanged. Same graceful-degradation as the rest
//! of the connector system: a failed connector keeps its fallback content
//! rather than breaking the element.

#[derive(Clone, Copy)]
enum Conversion {
    Fixed,
    Integer,
}

#[derive(Clone, Copy)]
enum Scale {
    Thousand,
    Million,
    Billion,
    Trillion,
}

impl Scale {
    fn from_char(c: char) -> Option<Scale> {
        match c {
            'K' => Some(Scale::Thousand),
            'M' => Some(Scale::Million),
            'B' => Some(Scale::Billion),
            'T' => Some(Scale::Trillion),
            _ => None,
        }
    }

    fn divisor(self) -> f64 {
        match self {
            Scale::Thousand => 1e3,
            Scale::Million => 1e6,
            Scale::Billion => 1e9,
            Scale::Trillion => 1e12,
        }
    }

    fn suffix(self) -> char {
        match self {
            Scale::Thousand => 'K',
            Scale::Million => 'M',
            Scale::Billion => 'B',
            Scale::Trillion => 'T',
        }
    }
}

struct Spec {
    thousands: bool,
    precision: Option<usize>,
    conversion: Conversion,
    scale: Option<Scale>,
}

fn parse_spec(spec: &str) -> Option<Spec> {
    let rest = spec.strip_prefix('%')?;
    let (thousands, rest) = match rest.strip_prefix(',') {
        Some(r) => (true, r),
        None => (false, rest),
    };

    let mut chars: Vec<char> = rest.chars().collect();
    let scale = match chars.last().copied().and_then(Scale::from_char) {
        Some(s) => {
            chars.pop();
            Some(s)
        }
        None => None,
    };

    let conv_char = chars.pop()?;
    let conversion = match conv_char {
        'f' => Conversion::Fixed,
        'd' | 'i' => Conversion::Integer,
        _ => return None,
    };

    let precision_part: String = chars.into_iter().collect();
    let precision = if precision_part.is_empty() {
        None
    } else {
        let digits = precision_part.strip_prefix('.')?;
        Some(digits.parse::<usize>().ok()?)
    };

    Some(Spec {
        thousands,
        precision,
        conversion,
        scale,
    })
}

fn add_thousands_separators(s: &str) -> String {
    let (sign, rest) = match s.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", s),
    };
    let (int_part, frac_part) = match rest.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };

    let len = int_part.len();
    let mut grouped = String::with_capacity(len + len / 3);
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }

    let mut out = format!("{sign}{grouped}");
    if let Some(f) = frac_part {
        out.push('.');
        out.push_str(f);
    }
    out
}

pub(crate) fn apply_number_format(value: &str, spec: &str) -> Option<String> {
    let number: f64 = value.trim().parse().ok()?;
    let spec = parse_spec(spec)?;

    let scaled = match spec.scale {
        Some(s) => number / s.divisor(),
        None => number,
    };

    let mut formatted = match spec.conversion {
        Conversion::Fixed => format!("{:.*}", spec.precision.unwrap_or(2), scaled),
        Conversion::Integer => format!("{}", scaled.round() as i64),
    };

    if spec.thousands {
        formatted = add_thousands_separators(&formatted);
    }
    if let Some(s) = spec.scale {
        formatted.push(s.suffix());
    }

    Some(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_fixed_precision() {
        assert_eq!(
            apply_number_format("1234.5678", "%.2f"),
            Some("1234.57".to_string())
        );
    }

    #[test]
    fn defaults_to_two_decimal_places_for_bare_f() {
        assert_eq!(
            apply_number_format("1234.5", "%f"),
            Some("1234.50".to_string())
        );
    }

    #[test]
    fn formats_rounded_integers() {
        assert_eq!(
            apply_number_format("1234.9", "%d"),
            Some("1235".to_string())
        );
        assert_eq!(
            apply_number_format("1234.4", "%i"),
            Some("1234".to_string())
        );
    }

    #[test]
    fn groups_with_thousands_separators() {
        assert_eq!(
            apply_number_format("1234567", "%,d"),
            Some("1,234,567".to_string())
        );
        assert_eq!(
            apply_number_format("1234567.891", "%,.1f"),
            Some("1,234,567.9".to_string())
        );
    }

    #[test]
    fn groups_negative_numbers_correctly() {
        assert_eq!(
            apply_number_format("-1234567", "%,d"),
            Some("-1,234,567".to_string())
        );
    }

    #[test]
    fn scales_and_appends_the_suffix_letter() {
        assert_eq!(
            apply_number_format("1000000", "%.0fK"),
            Some("1000K".to_string())
        );
        assert_eq!(
            apply_number_format("1000000", "%.1fM"),
            Some("1.0M".to_string())
        );
        assert_eq!(
            apply_number_format("2500000000", "%.2fB"),
            Some("2.50B".to_string())
        );
        assert_eq!(
            apply_number_format("1000000000000", "%dT"),
            Some("1T".to_string())
        );
    }

    #[test]
    fn combines_thousands_and_scale() {
        assert_eq!(
            apply_number_format("1234000", "%,.0fK"),
            Some("1,234K".to_string())
        );
    }

    #[test]
    fn returns_none_for_a_malformed_spec() {
        assert_eq!(apply_number_format("123", "not-a-spec"), None);
        assert_eq!(apply_number_format("123", "%"), None);
        assert_eq!(apply_number_format("123", "%x"), None);
        assert_eq!(apply_number_format("123", "%.abcf"), None);
    }

    #[test]
    fn returns_none_for_a_non_numeric_value() {
        assert_eq!(apply_number_format("v1.2.3", "%.2f"), None);
        assert_eq!(apply_number_format("passing", "%d"), None);
        assert_eq!(apply_number_format("", "%d"), None);
    }
}
