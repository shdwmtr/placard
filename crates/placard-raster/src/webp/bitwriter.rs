/// Packs bits LSB-first into a byte stream, the order VP8L's bitstream
/// uses throughout (unlike PNG/DEFLATE's byte-aligned stored blocks, which
/// never needed a bit packer at all).
pub(crate) struct BitWriter {
    bytes: Vec<u8>,
    acc: u64,
    nbits: u32,
}

impl BitWriter {
    pub(crate) fn new() -> Self {
        Self {
            bytes: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    /// Writes the low `n` bits of `value`, least-significant bit first.
    /// `n` may be 0..=32.
    pub(crate) fn write_bits(&mut self, value: u32, n: u32) {
        if n == 0 {
            return;
        }
        debug_assert!(n <= 32);
        debug_assert!(n == 32 || (value as u64) < (1u64 << n));

        self.acc |= (value as u64) << self.nbits;
        self.nbits += n;
        while self.nbits >= 8 {
            self.bytes.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }

    /// Flushes any partial trailing byte (zero-padded in the high bits) and
    /// returns the packed bytes.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.bytes.push((self.acc & 0xFF) as u8);
        }
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_bits_lsb_first_within_a_byte() {
        let mut w = BitWriter::new();
        w.write_bits(0b1, 1);
        w.write_bits(0b0, 1);
        w.write_bits(0b1, 1);
        // byte so far (LSB first): bit0=1, bit1=0, bit2=1 -> 0b0000_0101
        assert_eq!(w.finish(), vec![0b0000_0101]);
    }

    #[test]
    fn splits_a_value_across_a_byte_boundary() {
        let mut w = BitWriter::new();
        w.write_bits(0b1111_1111, 8);
        w.write_bits(0b101, 3);
        // first byte fully 1s; second byte holds the low 3 bits of the
        // second value in its low bits, rest zero.
        assert_eq!(w.finish(), vec![0xFF, 0b0000_0101]);
    }

    #[test]
    fn matches_a_hand_computed_multi_field_sequence() {
        let mut w = BitWriter::new();
        w.write_bits(0x2F, 8); // signature byte
        w.write_bits(3, 14); // width - 1
        w.write_bits(1, 14); // height - 1
        w.write_bits(0, 1); // alpha hint
        w.write_bits(0, 3); // version
        let bytes = w.finish();

        // Re-derive the same bits by hand via a plain bit accumulator.
        let mut acc: u64 = 0;
        let mut nbits = 0u32;
        for (value, n) in [(0x2Fu32, 8u32), (3, 14), (1, 14), (0, 1), (0, 3)] {
            acc |= (value as u64) << nbits;
            nbits += n;
        }
        let mut expected = Vec::new();
        while nbits > 0 {
            expected.push((acc & 0xFF) as u8);
            acc >>= 8;
            nbits = nbits.saturating_sub(8);
        }
        assert_eq!(bytes, expected);
    }

    #[test]
    fn empty_writer_produces_no_bytes() {
        let w = BitWriter::new();
        assert_eq!(w.finish(), Vec::<u8>::new());
    }

    #[test]
    fn writing_zero_bits_is_a_no_op() {
        let mut w = BitWriter::new();
        w.write_bits(5, 3);
        w.write_bits(0, 0);
        assert_eq!(w.finish(), vec![0b101]);
    }
}
