mod bitwriter;
mod huffman;
mod lz77;

use crate::canvas::Canvas;
use bitwriter::BitWriter;
use huffman::HuffmanTable;
use lz77::Token;
use std::path::Path;

const GREEN_ALPHABET_SIZE: usize = 256 + 24; // literal green values + LZ77 length codes
const CHANNEL_ALPHABET_SIZE: usize = 256;
const DISTANCE_ALPHABET_SIZE: usize = 40;

/// VP8L's *raw* distance value 1..=120 is reserved for a special
/// short-distance offset table (spatial neighbors like "one row up").
/// This encoder never uses it -- every backward reference is transmitted
/// as `flat_pixel_distance + 120`, which always falls past the special
/// table and decodes correctly via the plain `dist = raw - 120` branch.
/// The dominant case for this renderer's output, a run of the immediately
/// preceding pixel (flat distance 1, i.e. a solid-color run), lands on raw
/// distance 121 -- still a cheap, low prefix-code slot.
const DISTANCE_TABLE_OFFSET: u32 = 120;

fn pack_pixel(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

fn unpack_pixel(p: u32) -> (u8, u8, u8, u8) {
    ((p >> 24) as u8, (p >> 16) as u8, (p >> 8) as u8, p as u8)
}

struct Histograms {
    green: Vec<u32>,
    red: Vec<u32>,
    blue: Vec<u32>,
    alpha: Vec<u32>,
    distance: Vec<u32>,
}

fn build_histograms(tokens: &[Token]) -> Histograms {
    let mut green = vec![0u32; GREEN_ALPHABET_SIZE];
    let mut red = vec![0u32; CHANNEL_ALPHABET_SIZE];
    let mut blue = vec![0u32; CHANNEL_ALPHABET_SIZE];
    let mut alpha = vec![0u32; CHANNEL_ALPHABET_SIZE];
    let mut distance = vec![0u32; DISTANCE_ALPHABET_SIZE];

    for token in tokens {
        match *token {
            Token::Literal(pixel) => {
                let (r, g, b, a) = unpack_pixel(pixel);
                green[g as usize] += 1;
                red[r as usize] += 1;
                blue[b as usize] += 1;
                alpha[a as usize] += 1;
            }
            Token::Match {
                length,
                distance: dist,
            } => {
                let (length_prefix, _, _) = lz77::encode_prefix_value(length);
                green[256 + length_prefix as usize] += 1;
                let (dist_prefix, _, _) = lz77::encode_prefix_value(dist + DISTANCE_TABLE_OFFSET);
                distance[dist_prefix as usize] += 1;
            }
        }
    }

    Histograms {
        green,
        red,
        blue,
        alpha,
        distance,
    }
}

fn write_tokens(
    w: &mut BitWriter,
    tokens: &[Token],
    green: &HuffmanTable,
    red: &HuffmanTable,
    blue: &HuffmanTable,
    alpha: &HuffmanTable,
    distance: &HuffmanTable,
) {
    for token in tokens {
        match *token {
            Token::Literal(pixel) => {
                let (r, g, b, a) = unpack_pixel(pixel);
                green.write_symbol(w, g as usize);
                red.write_symbol(w, r as usize);
                blue.write_symbol(w, b as usize);
                alpha.write_symbol(w, a as usize);
            }
            Token::Match {
                length,
                distance: dist,
            } => {
                let (length_prefix, length_extra_bits, length_extra_value) =
                    lz77::encode_prefix_value(length);
                green.write_symbol(w, 256 + length_prefix as usize);
                w.write_bits(length_extra_value, length_extra_bits);

                let (dist_prefix, dist_extra_bits, dist_extra_value) =
                    lz77::encode_prefix_value(dist + DISTANCE_TABLE_OFFSET);
                distance.write_symbol(w, dist_prefix as usize);
                w.write_bits(dist_extra_value, dist_extra_bits);
            }
        }
    }
}

/// Encodes `canvas` as a lossless WebP (VP8L). This project's encoder
/// always: skips all four optional color transforms, skips the color
/// cache, and uses a single global Huffman code group (no meta-Huffman
/// image) -- see the module-level docs in the crate's plan for why this
/// subset is fully spec-valid and where the real compression comes from
/// (Huffman entropy coding plus LZ77-style backward references).
pub fn encode(canvas: &Canvas) -> Result<Vec<u8>, String> {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        return Err("cannot encode a zero-sized canvas as WebP".to_string());
    }
    if width > 16384 || height > 16384 {
        return Err(format!(
            "canvas {width}x{height} exceeds VP8L's maximum dimension of 16384px per side"
        ));
    }

    let pixels_bytes = canvas.pixels();
    let mut pixels = Vec::with_capacity((width * height) as usize);
    let mut alpha_is_used = false;
    for chunk in pixels_bytes.chunks_exact(4) {
        pixels.push(pack_pixel(chunk[0], chunk[1], chunk[2], chunk[3]));
        if chunk[3] != 255 {
            alpha_is_used = true;
        }
    }

    let tokens = lz77::find_tokens(&pixels);
    let histograms = build_histograms(&tokens);

    let green_table = HuffmanTable::from_histogram(&histograms.green);
    let red_table = HuffmanTable::from_histogram(&histograms.red);
    let blue_table = HuffmanTable::from_histogram(&histograms.blue);
    let alpha_table = HuffmanTable::from_histogram(&histograms.alpha);
    let distance_table = HuffmanTable::from_histogram(&histograms.distance);

    let mut w = BitWriter::new();
    w.write_bits(0x2F, 8); // signature
    w.write_bits(width - 1, 14);
    w.write_bits(height - 1, 14);
    w.write_bits(alpha_is_used as u32, 1);
    w.write_bits(0, 3); // version_number

    w.write_bits(0, 1); // no transforms
    w.write_bits(0, 1); // color_cache_present = 0
    w.write_bits(0, 1); // meta_prefix_codes_present = 0 (single global Huffman group)

    huffman::write_huffman_code(&mut w, &green_table, GREEN_ALPHABET_SIZE);
    huffman::write_huffman_code(&mut w, &red_table, CHANNEL_ALPHABET_SIZE);
    huffman::write_huffman_code(&mut w, &blue_table, CHANNEL_ALPHABET_SIZE);
    huffman::write_huffman_code(&mut w, &alpha_table, CHANNEL_ALPHABET_SIZE);
    huffman::write_huffman_code(&mut w, &distance_table, DISTANCE_ALPHABET_SIZE);

    write_tokens(
        &mut w,
        &tokens,
        &green_table,
        &red_table,
        &blue_table,
        &alpha_table,
        &distance_table,
    );

    let vp8l_data = w.finish();
    Ok(wrap_riff_container(&vp8l_data))
}

fn wrap_riff_container(vp8l_data: &[u8]) -> Vec<u8> {
    let padded_len = vp8l_data.len() + (vp8l_data.len() & 1);
    let riff_size = 4 + 8 + padded_len; // "WEBP" + ("VP8L" + u32 size) + padded data

    let mut out = Vec::with_capacity(8 + riff_size);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(riff_size as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(b"VP8L");
    out.extend_from_slice(&(vp8l_data.len() as u32).to_le_bytes());
    out.extend_from_slice(vp8l_data);
    if vp8l_data.len() & 1 == 1 {
        out.push(0);
    }
    out
}

/// Writes `canvas` to `path` as a lossless WebP.
pub fn write<P: AsRef<Path>>(canvas: &Canvas, path: P) -> Result<(), String> {
    let bytes = encode(canvas)?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Color;

    #[test]
    fn encodes_a_solid_color_canvas() {
        let mut canvas = Canvas::new(4, 4);
        canvas.fill(Color::rgba(10, 20, 30, 255));
        let bytes = encode(&canvas).expect("should encode");
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
        assert_eq!(&bytes[12..16], b"VP8L");
    }

    #[test]
    fn riff_size_field_matches_actual_payload() {
        let mut canvas = Canvas::new(10, 3);
        canvas.fill(Color::rgba(1, 2, 3, 255));
        let bytes = encode(&canvas).expect("should encode");
        let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff_size, bytes.len() - 8);
    }

    #[test]
    fn rejects_oversized_dimensions() {
        let canvas = Canvas::new(20000, 1);
        assert!(encode(&canvas).is_err());
    }

    #[test]
    fn rejects_zero_sized_canvas() {
        let canvas = Canvas::new(0, 5);
        assert!(encode(&canvas).is_err());
    }

    #[test]
    fn encodes_a_multi_color_checkerboard() {
        let mut canvas = Canvas::new(9, 5);
        for y in 0..5 {
            for x in 0..9 {
                let on = (x + y) % 2 == 0;
                let c = if on {
                    Color::rgba(200, 30, 90, 255)
                } else {
                    Color::rgba(10, 220, 40, 128)
                };
                canvas.fill_rect(x, y, 1, 1, c);
            }
        }
        let bytes = encode(&canvas).expect("should encode");
        assert_eq!(&bytes[0..4], b"RIFF");
    }
}
