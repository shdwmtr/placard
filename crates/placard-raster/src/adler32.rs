const MOD_ADLER: u32 = 65521;

/// Folds `data` into a running `(a, b)` state, so a checksum can be built up
/// incrementally over pieces of a buffer that's never assembled in one
/// place -- `a = 1, b = 0` is the correct starting state (matching the
/// standard adler32 checksum of an empty input).
pub fn adler32_update(a: &mut u32, b: &mut u32, data: &[u8]) {
    for &byte in data {
        *a = (*a + byte as u32) % MOD_ADLER;
        *b = (*b + *a) % MOD_ADLER;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adler32(data: &[u8]) -> u32 {
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        adler32_update(&mut a, &mut b, data);
        (b << 16) | a
    }

    #[test]
    fn matches_wikipedia_example() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
    }

    #[test]
    fn empty_input_is_one() {
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn incremental_updates_match_one_shot() {
        let (mut a1, mut b1) = (1u32, 0u32);
        adler32_update(&mut a1, &mut b1, b"Wiki");
        adler32_update(&mut a1, &mut b1, b"pedia");
        assert_eq!((b1 << 16) | a1, adler32(b"Wikipedia"));
    }
}
