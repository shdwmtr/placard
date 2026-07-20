const POLY: u32 = 0xEDB88320;

const fn make_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0usize;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { POLY ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

const TABLE: [u32; 256] = make_table();

/// Folds `data` into a running, pre-final-XOR CRC state (start at
/// `0xFFFFFFFF`), so a checksum can be built up over pieces of a buffer
/// that's streamed out rather than held in memory as a whole. Apply the
/// final `^ 0xFFFFFFFF` only once, after the last update.
pub fn crc32_update(crc: &mut u32, data: &[u8]) {
    for &byte in data {
        let idx = ((*crc ^ byte as u32) & 0xFF) as usize;
        *crc = TABLE[idx] ^ (*crc >> 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFFFFFFu32;
        crc32_update(&mut crc, data);
        crc ^ 0xFFFFFFFF
    }

    #[test]
    fn matches_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn empty_input_is_zero() {
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn incremental_updates_match_one_shot() {
        let mut crc = 0xFFFFFFFFu32;
        crc32_update(&mut crc, b"12345");
        crc32_update(&mut crc, b"6789");
        assert_eq!(crc ^ 0xFFFFFFFF, crc32(b"123456789"));
    }
}
