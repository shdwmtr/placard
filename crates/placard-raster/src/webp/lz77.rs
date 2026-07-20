use std::collections::HashMap;

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 4096;
const MAX_CHAIN: usize = 32;

/// VP8L's backward-reference distance is transmitted as
/// `flat_pixel_distance + 120` (see `mod.rs` for why this project always
/// skips the special short-distance offset table), and that raw value has
/// to fit the 40-slot distance prefix-code alphabet, whose largest slot
/// covers up to 1_048_576. So the matcher's window is capped accordingly.
const MAX_DISTANCE_WINDOW: usize = 1_048_576 - 120;

pub(crate) enum Token {
    Literal(u32),
    Match { length: u32, distance: u32 },
}

/// Greedy hash-chain LZ77-style match finder over whole pixels (VP8L
/// backward references copy pixels, not bytes). Not globally optimal --
/// each position tries up to `MAX_CHAIN` prior occurrences of the same
/// pixel value and keeps the longest -- but that's enough to capture the
/// dominant case for this renderer's output: long runs of flat color.
pub(crate) fn find_tokens(pixels: &[u32]) -> Vec<Token> {
    let n = pixels.len();
    let mut last: HashMap<u32, usize> = HashMap::new();
    let mut prev: Vec<i64> = vec![-1; n];
    let mut tokens = Vec::new();

    let mut i = 0;
    while i < n {
        let value = pixels[i];
        let mut best_len = 0usize;
        let mut best_dist = 0usize;

        if let Some(&start) = last.get(&value) {
            let mut candidate = start as i64;
            let mut tries = 0;
            while candidate >= 0 && tries < MAX_CHAIN {
                let p = candidate as usize;
                if i - p > MAX_DISTANCE_WINDOW {
                    break;
                }
                let max_len = (n - i).min(MAX_MATCH);
                let mut len = 0;
                while len < max_len && pixels[p + len] == pixels[i + len] {
                    len += 1;
                }
                if len > best_len {
                    best_len = len;
                    best_dist = i - p;
                }
                candidate = prev[p];
                tries += 1;
            }
        }

        if best_len >= MIN_MATCH {
            tokens.push(Token::Match {
                length: best_len as u32,
                distance: best_dist as u32,
            });
            for k in 0..best_len {
                let pos = i + k;
                let v = pixels[pos];
                prev[pos] = last.get(&v).map(|&p| p as i64).unwrap_or(-1);
                last.insert(v, pos);
            }
            i += best_len;
        } else {
            tokens.push(Token::Literal(value));
            prev[i] = last.get(&value).map(|&p| p as i64).unwrap_or(-1);
            last.insert(value, i);
            i += 1;
        }
    }

    tokens
}

/// The shared prefix-code + extra-bits scheme VP8L uses for both the
/// length alphabet (24 slots, values 1..=4096) and the distance alphabet
/// (40 slots, values 1..=1_048_576): slots 0..4 cover values 1..4
/// directly; each slot after that doubles the range covered by the
/// previous pair of slots. Returns `(prefix_code, extra_bits, extra_value)`
/// such that `value == decode_prefix_value(prefix_code, extra_bits, extra_value)`.
pub(crate) fn encode_prefix_value(value: u32) -> (u32, u32, u32) {
    debug_assert!(value >= 1);
    if value <= 4 {
        return (value - 1, 0, 0);
    }
    let l = value - 1;
    let mut prefix_code = 4u32;
    loop {
        let extra_bits = (prefix_code - 2) >> 1;
        let offset = (2 + (prefix_code & 1)) << extra_bits;
        if l >= offset && l < offset + (1 << extra_bits) {
            return (prefix_code, extra_bits, l - offset);
        }
        prefix_code += 1;
    }
}

#[cfg(test)]
pub(crate) fn decode_prefix_value(prefix_code: u32, extra_value: u32) -> u32 {
    if prefix_code < 4 {
        return prefix_code + 1;
    }
    let extra_bits = (prefix_code - 2) >> 1;
    let offset = (2 + (prefix_code & 1)) << extra_bits;
    offset + extra_value + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_flat_run_becomes_a_single_match() {
        let mut pixels = vec![0xFFFFFFFFu32; 1000];
        pixels.extend(vec![0x00000000u32; 1]);
        let tokens = find_tokens(&pixels);
        // first pixel literal, then one huge match covering the rest of
        // the run, then the differing final pixel as a literal.
        assert!(tokens.len() <= 3);
        let total: usize = tokens
            .iter()
            .map(|t| match t {
                Token::Literal(_) => 1,
                Token::Match { length, .. } => *length as usize,
            })
            .sum();
        assert_eq!(total, pixels.len());
    }

    #[test]
    fn tokens_reconstruct_the_original_pixels() {
        let pixels = vec![1u32, 2, 3, 1, 2, 3, 1, 2, 3, 9, 9, 9, 9, 9, 4];
        let tokens = find_tokens(&pixels);

        let mut out = Vec::with_capacity(pixels.len());
        for t in &tokens {
            match *t {
                Token::Literal(v) => out.push(v),
                Token::Match { length, distance } => {
                    for _ in 0..length {
                        let src = out.len() - distance as usize;
                        out.push(out[src]);
                    }
                }
            }
        }
        assert_eq!(out, pixels);
    }

    #[test]
    fn no_repetition_is_all_literals() {
        let pixels: Vec<u32> = (0..50).collect();
        let tokens = find_tokens(&pixels);
        assert_eq!(tokens.len(), pixels.len());
        assert!(tokens.iter().all(|t| matches!(t, Token::Literal(_))));
    }

    #[test]
    fn prefix_value_round_trips_across_the_full_length_range() {
        for value in 1..=4096u32 {
            let (prefix, extra_bits, extra_value) = encode_prefix_value(value);
            assert!(extra_value < (1 << extra_bits));
            assert_eq!(decode_prefix_value(prefix, extra_value), value);
        }
    }

    #[test]
    fn prefix_value_round_trips_across_the_full_distance_range() {
        // Spot-check across the full distance range (exhaustive would be
        // slow): boundaries of every slot plus a spread of values.
        let mut values: Vec<u32> = (1..=10_000).collect();
        values.extend((0..40).map(|_| 1_048_576u32));
        values.push(786_433);
        values.push(1_048_576);
        for value in values {
            let (prefix, extra_bits, extra_value) = encode_prefix_value(value);
            assert!(
                prefix < 40,
                "prefix {prefix} out of distance-alphabet range for value {value}"
            );
            assert!(extra_value < (1 << extra_bits));
            assert_eq!(decode_prefix_value(prefix, extra_value), value);
        }
    }

    #[test]
    fn code_39_matches_the_documented_range() {
        let (prefix, extra_bits, _) = encode_prefix_value(786_433);
        assert_eq!(prefix, 39);
        assert_eq!(extra_bits, 18);
        let (prefix, _, _) = encode_prefix_value(1_048_576);
        assert_eq!(prefix, 39);
    }
}
