use crate::webp::bitwriter::BitWriter;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const MAX_CODE_LENGTH: u32 = 15;

/// The meta-alphabet (over the 19 code-length symbols, used to transmit
/// every other table's code lengths) has its own code lengths transmitted
/// via a hard 3-bit field (`code_length_code_lengths[...] = ReadBits(3)`
/// per spec), so its own Huffman tree must never exceed depth 7 -- a
/// separate, smaller limit than the 15-bit cap that applies everywhere
/// else.
const MAX_META_CODE_LENGTH: u32 = 7;

/// A canonical Huffman code table: `lengths[symbol]` is the code length in
/// bits (0 if the symbol is unused), `codes[symbol]` is its canonical code
/// value (natural bit order, i.e. bit 0 of `codes[symbol]` is the *first*
/// bit a decoder reads -- see `write_symbol`, which reverses this into the
/// bitstream's LSB-first packing).
pub(crate) struct HuffmanTable {
    lengths: Vec<u8>,
    codes: Vec<u16>,
    /// True when exactly one symbol in the alphabet is used. Per spec, a
    /// single-leaf-node tree is a complete tree (transmitted with that one
    /// symbol's length marked as 1), but reading/writing a symbol from it
    /// consumes *zero* bits in the actual data stream -- there is no choice
    /// to encode. Getting this wrong desyncs the entire bitstream from the
    /// first such symbol onward.
    single_symbol: bool,
}

struct HeapNode {
    weight: u64,
    // Tie-break by insertion order so the heap ordering (and therefore the
    // resulting code lengths for equal-weight symbols) is deterministic.
    order: u32,
    node: Tree,
}

impl PartialEq for HeapNode {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight && self.order == other.order
    }
}
impl Eq for HeapNode {}
impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.weight, self.order).cmp(&(other.weight, other.order))
    }
}

enum Tree {
    Leaf(u32),
    Node(Box<Tree>, Box<Tree>),
}

fn assign_depths(tree: &Tree, depth: u8, out: &mut [u8]) {
    match tree {
        Tree::Leaf(symbol) => out[*symbol as usize] = depth.max(1),
        Tree::Node(a, b) => {
            assign_depths(a, depth + 1, out);
            assign_depths(b, depth + 1, out);
        }
    }
}

/// Computes code lengths (0 = unused) for `histogram` via a standard
/// priority-queue Huffman construction, then limits the maximum length to
/// `limit` bits.
fn code_lengths_from_histogram(histogram: &[u32], limit: u32) -> Vec<u8> {
    let alphabet_size = histogram.len();
    let mut lengths = vec![0u8; alphabet_size];

    let used: Vec<(usize, u32)> = histogram
        .iter()
        .enumerate()
        .filter(|&(_, &c)| c > 0)
        .map(|(i, &c)| (i, c))
        .collect();

    if used.is_empty() {
        // Per spec: an entirely-unused alphabet (e.g. the distance table
        // when an image has no backward references) must still be
        // transmitted as a single-leaf-node tree for symbol 0, not as
        // all-zero lengths.
        lengths[0] = 1;
        return lengths;
    }
    if used.len() == 1 {
        lengths[used[0].0] = 1;
        return lengths;
    }

    let mut heap: BinaryHeap<Reverse<HeapNode>> = BinaryHeap::new();
    for (order, &(symbol, count)) in used.iter().enumerate() {
        heap.push(Reverse(HeapNode {
            weight: count as u64,
            order: order as u32,
            node: Tree::Leaf(symbol as u32),
        }));
    }

    let mut order = used.len() as u32;
    while heap.len() > 1 {
        let Reverse(a) = heap.pop().unwrap();
        let Reverse(b) = heap.pop().unwrap();
        heap.push(Reverse(HeapNode {
            weight: a.weight + b.weight,
            order,
            node: Tree::Node(Box::new(a.node), Box::new(b.node)),
        }));
        order += 1;
    }

    let Reverse(root) = heap.pop().unwrap();
    assign_depths(&root.node, 0, &mut lengths);

    limit_code_lengths(&mut lengths, limit);

    debug_assert!(
        {
            let sum: f64 = lengths
                .iter()
                .filter(|&&l| l > 0)
                .map(|&l| 2f64.powi(-(l as i32)))
                .sum();
            (sum - 1.0).abs() < 1e-6
        },
        "Kraft sum not 1.0 after length limiting"
    );

    lengths
}

/// Classic overflow-redistribution technique for bounding canonical
/// Huffman code lengths to `limit` bits while preserving the Kraft
/// inequality (each promotion of an over-length code up by one level is
/// paid for by demoting a code at the next available shorter length down
/// into the freed slot).
fn limit_code_lengths(lengths: &mut [u8], limit: u32) {
    let max_len = *lengths.iter().max().unwrap_or(&0) as usize;
    if max_len as u32 <= limit {
        return;
    }

    let mut count = vec![0u32; max_len + 1];
    for &l in lengths.iter() {
        if l > 0 {
            count[l as usize] += 1;
        }
    }

    let limit = limit as usize;
    for len in (limit + 1..=max_len).rev() {
        while count[len] > 0 {
            let mut j = len - 2;
            while j > 0 && count[j] == 0 {
                j -= 1;
            }
            count[len] -= 2;
            count[len - 1] += 1;
            count[j + 1] += 2;
            count[j] -= 1;
        }
    }

    // Reassign lengths to symbols, longest-first among the pool of used
    // symbols sorted by their original length (stable so ties keep a
    // deterministic, symbol-index order).
    let mut symbols: Vec<usize> = (0..lengths.len()).filter(|&i| lengths[i] > 0).collect();
    symbols.sort_by_key(|&i| Reverse(lengths[i]));

    let mut idx = 0;
    for len in (1..=limit).rev() {
        for _ in 0..count[len] {
            lengths[symbols[idx]] = len as u8;
            idx += 1;
        }
    }
    debug_assert_eq!(idx, symbols.len());
}

/// Assigns canonical codes from code lengths: symbols are ordered first by
/// increasing length, then by increasing symbol index, and codes increase
/// in that same order (the standard canonical Huffman convention).
fn canonical_codes(lengths: &[u8]) -> Vec<u16> {
    let max_len = *lengths.iter().max().unwrap_or(&0) as usize;
    let mut bl_count = vec![0u32; max_len + 1];
    for &l in lengths {
        if l > 0 {
            bl_count[l as usize] += 1;
        }
    }

    let mut next_code = vec![0u32; max_len + 2];
    let mut code = 0u32;
    for len in 1..=max_len {
        code = (code + bl_count[len - 1]) << 1;
        next_code[len] = code;
    }

    let mut codes = vec![0u16; lengths.len()];
    for (symbol, &len) in lengths.iter().enumerate() {
        if len > 0 {
            codes[symbol] = next_code[len as usize] as u16;
            next_code[len as usize] += 1;
        }
    }
    codes
}

impl HuffmanTable {
    pub(crate) fn from_histogram(histogram: &[u32]) -> Self {
        Self::from_histogram_limited(histogram, MAX_CODE_LENGTH)
    }

    /// Builds a table whose own code lengths never exceed `limit` bits --
    /// used for the meta-alphabet, whose lengths are transmitted via a
    /// fixed-width field too narrow for the general 15-bit cap.
    fn from_histogram_limited(histogram: &[u32], limit: u32) -> Self {
        let lengths = code_lengths_from_histogram(histogram, limit);
        let codes = canonical_codes(&lengths);
        let single_symbol = lengths.iter().filter(|&&l| l > 0).count() == 1;
        Self {
            lengths,
            codes,
            single_symbol,
        }
    }

    pub(crate) fn len_of(&self, symbol: usize) -> u8 {
        self.lengths[symbol]
    }

    /// Writes `symbol`'s canonical code into the bitstream. Canonical codes
    /// are naturally MSB-first (bit 0 of `code` is the first bit read by a
    /// decoder), so the value is bit-reversed before handing it to the
    /// LSB-first `BitWriter`. Writes nothing at all for a single-leaf-node
    /// table (see the `single_symbol` field doc).
    pub(crate) fn write_symbol(&self, w: &mut BitWriter, symbol: usize) {
        if self.single_symbol {
            return;
        }
        let len = self.lengths[symbol];
        debug_assert!(len > 0, "attempted to emit an unused symbol");
        let code = self.codes[symbol];
        let reversed = reverse_bits(code, len);
        w.write_bits(reversed as u32, len as u32);
    }
}

fn reverse_bits(value: u16, n: u8) -> u16 {
    let mut v = value;
    let mut r = 0u16;
    for _ in 0..n {
        r = (r << 1) | (v & 1);
        v >>= 1;
    }
    r
}

/// The fixed order VP8L transmits the 19-symbol code-length alphabet's own
/// code lengths in.
const CODE_LENGTH_CODE_ORDER: [usize; 19] = [
    17, 18, 0, 1, 2, 3, 4, 5, 16, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

/// Transmits `table` (a code-length table over `alphabet_size` symbols) via
/// VP8L's "normal" code-length coding path: a nested Huffman code over the
/// 19-symbol code-length alphabet, describing every symbol's length
/// literally (0..=15). This project's encoder never emits the run-length
/// repeat symbols (16/17/18) -- always encoding each length literally is
/// simpler and, since alphabets here are at most 280 symbols, costs at
/// most a few hundred bits of fixed overhead versus using them, which is
/// negligible next to actual pixel-data savings.
pub(crate) fn write_huffman_code(w: &mut BitWriter, table: &HuffmanTable, alphabet_size: usize) {
    let mut meta_histogram = [0u32; 19];
    for symbol in 0..alphabet_size {
        meta_histogram[table.len_of(symbol) as usize] += 1;
    }
    let meta_table = HuffmanTable::from_histogram_limited(&meta_histogram, MAX_META_CODE_LENGTH);

    w.write_bits(0, 1); // is_simple_code_lengths_code = 0 (always use the normal path)

    w.write_bits(19 - 4, 4); // num_code_lengths = 19
    for &sym in CODE_LENGTH_CODE_ORDER.iter() {
        w.write_bits(meta_table.len_of(sym) as u32, 3);
    }

    w.write_bits(0, 1); // use_max_symbol_flag = 0 -> max_symbol = alphabet_size

    for symbol in 0..alphabet_size {
        meta_table.write_symbol(w, table.len_of(symbol) as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kraft_sum(lengths: &[u8]) -> f64 {
        lengths
            .iter()
            .filter(|&&l| l > 0)
            .map(|&l| 2f64.powi(-(l as i32)))
            .sum()
    }

    fn kraft_ok(lengths: &[u8]) -> bool {
        kraft_sum(lengths) <= 1.0 + 1e-9
    }

    #[test]
    fn single_used_symbol_gets_a_one_bit_code() {
        let mut hist = vec![0u32; 8];
        hist[3] = 100;
        let lengths = code_lengths_from_histogram(&hist, MAX_CODE_LENGTH);
        assert_eq!(lengths[3], 1);
        assert!(lengths.iter().enumerate().all(|(i, &l)| i == 3 || l == 0));
    }

    #[test]
    fn two_used_symbols_each_get_a_one_bit_code() {
        let mut hist = vec![0u32; 5];
        hist[0] = 10;
        hist[4] = 1;
        let lengths = code_lengths_from_histogram(&hist, MAX_CODE_LENGTH);
        assert_eq!(lengths[0], 1);
        assert_eq!(lengths[4], 1);
        assert!(kraft_ok(&lengths));
    }

    #[test]
    fn textbook_five_symbol_histogram_matches_known_optimal_lengths() {
        // Classic example (counts 1,1,2,3,5): greedy-merge order is
        // (A+B)->2, (AB+C)->4, (D+ABC)->7, (ABCD+E)->12, giving depths
        // E=1, D=2, C=3, A=B=4 -- a complete tree (Kraft sum exactly 1).
        let hist = vec![1u32, 1, 2, 3, 5];
        let lengths = code_lengths_from_histogram(&hist, MAX_CODE_LENGTH);
        let mut sorted = lengths.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 4]);
        assert!((kraft_sum(&lengths) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn skewed_histogram_is_length_limited_to_fifteen_bits() {
        // Fibonacci-like counts force an unlimited Huffman tree deeper
        // than 15 for the rarest symbols; confirm the limiter caps it
        // while keeping a valid (Kraft-satisfying) prefix code.
        let mut counts = vec![1u64, 1];
        while counts.len() < 40 {
            let n = counts.len();
            counts.push(counts[n - 1] + counts[n - 2]);
        }
        let hist: Vec<u32> = counts
            .iter()
            .map(|&c| c.min(u32::MAX as u64) as u32)
            .collect();
        let lengths = code_lengths_from_histogram(&hist, MAX_CODE_LENGTH);
        assert!(lengths.iter().all(|&l| l as u32 <= MAX_CODE_LENGTH));
        assert!(kraft_ok(&lengths));
    }

    #[test]
    fn canonical_codes_increase_with_symbol_index_at_equal_length() {
        let hist = vec![5u32, 5, 5, 5]; // 4 equally likely symbols -> all length 2
        let lengths = code_lengths_from_histogram(&hist, MAX_CODE_LENGTH);
        assert!(lengths.iter().all(|&l| l == 2));
        let codes = canonical_codes(&lengths);
        let mut sorted_codes = codes.clone();
        sorted_codes.sort();
        assert_eq!(
            codes, sorted_codes,
            "codes should increase with symbol index"
        );
        assert_eq!(codes, vec![0, 1, 2, 3]);
    }

    #[test]
    fn reverse_bits_round_trips() {
        assert_eq!(reverse_bits(0b101, 3), 0b101);
        assert_eq!(reverse_bits(0b100, 3), 0b001);
        assert_eq!(reverse_bits(0b1100, 4), 0b0011);
    }
}
