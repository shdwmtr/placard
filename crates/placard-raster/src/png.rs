use crate::adler32::adler32_update;
use crate::canvas::Canvas;
use crate::crc32::crc32_update;
use std::io::{self, Write};
use std::path::Path;

const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const MAX_STORED_BLOCK: usize = 65535;

/// Writes `canvas` to `path` as a PNG, streaming the encoded bytes straight
/// into a buffered file writer -- no full-image-sized buffer is held in
/// memory at all here, unlike `encode`.
pub fn write<P: AsRef<Path>>(canvas: &Canvas, path: P) -> io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = io::BufWriter::new(file);
    encode_to_writer(canvas, &mut writer)
}

/// Encodes `canvas` as a PNG into an in-memory buffer. Prefer `write` (or
/// `encode_to_writer` directly) when the destination is a file or socket --
/// this still has to hold the whole encoded image as one `Vec<u8>`, since
/// the caller gets it back as owned bytes.
pub fn encode(canvas: &Canvas) -> Vec<u8> {
    let raw_len = canvas.height() as usize * (canvas.width() as usize * 4 + 1);
    let mut out = Vec::with_capacity(64 + zlib_stored_len(raw_len));
    encode_to_writer(canvas, &mut out).expect("writing to a Vec<u8> is infallible");
    out
}

/// Encodes `canvas` as a PNG directly into `writer` in one pass. Beyond the
/// canvas itself (owned by the caller) and small, fixed-size scratch state,
/// nothing here holds a full copy of the image -- not the filtered
/// scanlines, not the zlib-framed data, not the encoded output. Every
/// stored-block length is computed analytically up front (see
/// `zlib_stored_len`), since these are uncompressed "stored" deflate blocks
/// of fully predictable size, so each chunk's length can be written before
/// its data the way the PNG format expects, without buffering the data
/// first just to measure it.
pub fn encode_to_writer<W: Write>(canvas: &Canvas, writer: &mut W) -> io::Result<()> {
    let width = canvas.width() as usize;
    let height = canvas.height() as usize;
    let stride = width * 4;
    let pixels = canvas.pixels();
    let raw_len = height * (stride + 1);
    let idat_len = zlib_stored_len(raw_len);

    writer.write_all(&PNG_SIGNATURE)?;
    write_chunk(writer, b"IHDR", &ihdr_data(canvas))?;
    write_idat_chunk(writer, pixels, stride, raw_len, idat_len)?;
    write_chunk(writer, b"IEND", &[])?;
    Ok(())
}

fn ihdr_data(canvas: &Canvas) -> [u8; 13] {
    let mut data = [0u8; 13];
    data[0..4].copy_from_slice(&canvas.width().to_be_bytes());
    data[4..8].copy_from_slice(&canvas.height().to_be_bytes());
    data[8] = 8; // bit depth
    data[9] = 6; // color type: truecolor with alpha
    data
}

fn write_chunk(writer: &mut impl Write, chunk_type: &[u8; 4], data: &[u8]) -> io::Result<()> {
    writer.write_all(&(data.len() as u32).to_be_bytes())?;
    let mut crc = 0xFFFFFFFFu32;
    writer.write_all(chunk_type)?;
    crc32_update(&mut crc, chunk_type);
    writer.write_all(data)?;
    crc32_update(&mut crc, data);
    writer.write_all(&(crc ^ 0xFFFFFFFF).to_be_bytes())?;
    Ok(())
}

/// Total byte length of the zlib-wrapped stored-deflate-blocks encoding
/// (2-byte header + stored blocks of up to `MAX_STORED_BLOCK` bytes, each
/// with a 5-byte header + 4-byte adler32 trailer) of `raw_len` bytes of
/// input.
fn zlib_stored_len(raw_len: usize) -> usize {
    let blocks_len = if raw_len == 0 {
        5
    } else {
        raw_len + raw_len.div_ceil(MAX_STORED_BLOCK) * 5
    };
    2 + blocks_len + 4
}

/// Writes the IDAT chunk (length, type, zlib-wrapped filtered scanlines,
/// CRC) to `writer`. The scanline data -- a filter-type-0 byte followed by
/// `stride` pixel bytes, per row -- is never materialized as its own
/// buffer: each stored block's bytes are copied straight from `pixels` (or
/// synthesized, for filter bytes) to `writer`.
fn write_idat_chunk(
    writer: &mut impl Write,
    pixels: &[u8],
    stride: usize,
    raw_len: usize,
    idat_len: usize,
) -> io::Result<()> {
    writer.write_all(&(idat_len as u32).to_be_bytes())?;

    let mut crc = 0xFFFFFFFFu32;
    writer.write_all(b"IDAT")?;
    crc32_update(&mut crc, b"IDAT");

    let zlib_header = [0x78u8, 0x01u8];
    writer.write_all(&zlib_header)?;
    crc32_update(&mut crc, &zlib_header);

    let mut a: u32 = 1;
    let mut b: u32 = 0;

    if raw_len == 0 {
        let header = [0x01u8, 0, 0, 0xFF, 0xFF];
        writer.write_all(&header)?;
        crc32_update(&mut crc, &header);
    } else {
        let mut offset = 0;
        while offset < raw_len {
            let end = (offset + MAX_STORED_BLOCK).min(raw_len);
            let len = end - offset;
            let is_final = end == raw_len;

            let mut header = [0u8; 5];
            header[0] = if is_final { 0x01 } else { 0x00 };
            header[1..3].copy_from_slice(&(len as u16).to_le_bytes());
            header[3..5].copy_from_slice(&(!(len as u16)).to_le_bytes());
            writer.write_all(&header)?;
            crc32_update(&mut crc, &header);

            copy_virtual_scanlines(
                pixels, stride, offset, len, writer, &mut crc, &mut a, &mut b,
            )?;

            offset = end;
        }
    }

    let adler = ((b << 16) | a).to_be_bytes();
    writer.write_all(&adler)?;
    crc32_update(&mut crc, &adler);

    writer.write_all(&(crc ^ 0xFFFFFFFF).to_be_bytes())?;
    Ok(())
}

/// Writes `len` bytes starting at virtual offset `virtual_pos` of the
/// conceptual "filter byte + `stride` pixel bytes, per row" scanline stream
/// to `writer`, folding the same bytes into the running adler32 (data
/// only) and crc32 (this chunk's type + all its data) states as they go.
fn copy_virtual_scanlines(
    pixels: &[u8],
    stride: usize,
    mut virtual_pos: usize,
    mut remaining: usize,
    writer: &mut impl Write,
    crc: &mut u32,
    a: &mut u32,
    b: &mut u32,
) -> io::Result<()> {
    let row_len = stride + 1;
    while remaining > 0 {
        let row = virtual_pos / row_len;
        let pos_in_row = virtual_pos % row_len;

        if pos_in_row == 0 {
            writer.write_all(&[0])?;
            crc32_update(crc, &[0]);
            adler32_update(a, b, &[0]);
            virtual_pos += 1;
            remaining -= 1;
        } else {
            let pixel_offset = pos_in_row - 1;
            let row_start = row * stride;
            let available = stride - pixel_offset;
            let take = available.min(remaining);
            let src = &pixels[row_start + pixel_offset..row_start + pixel_offset + take];
            writer.write_all(src)?;
            crc32_update(crc, src);
            adler32_update(a, b, src);
            virtual_pos += take;
            remaining -= take;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Color;

    /// A from-first-principles re-implementation of the original encoder
    /// (materializing the whole filtered scanline buffer, then the whole
    /// zlib-framed buffer, as separate `Vec`s) kept only in this test as a
    /// reference to check the streaming encoder above against byte-for-byte.
    fn reference_encode(canvas: &Canvas) -> Vec<u8> {
        fn reference_zlib_stored(data: &[u8]) -> Vec<u8> {
            let mut out = Vec::new();
            out.push(0x78);
            out.push(0x01);
            if data.is_empty() {
                out.push(0x01);
                out.extend_from_slice(&0u16.to_le_bytes());
                out.extend_from_slice(&0xFFFFu16.to_le_bytes());
            } else {
                let mut offset = 0;
                while offset < data.len() {
                    let end = (offset + MAX_STORED_BLOCK).min(data.len());
                    let chunk = &data[offset..end];
                    let is_final = end == data.len();
                    out.push(if is_final { 0x01 } else { 0x00 });
                    let len = chunk.len() as u16;
                    out.extend_from_slice(&len.to_le_bytes());
                    out.extend_from_slice(&(!len).to_le_bytes());
                    out.extend_from_slice(chunk);
                    offset = end;
                }
            }
            let mut a = 1u32;
            let mut b = 0u32;
            adler32_update(&mut a, &mut b, data);
            out.extend_from_slice(&((b << 16) | a).to_be_bytes());
            out
        }

        fn reference_write_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let start = out.len();
            out.extend_from_slice(chunk_type);
            out.extend_from_slice(data);
            let mut crc = 0xFFFFFFFFu32;
            crc32_update(&mut crc, &out[start..]);
            out.extend_from_slice(&(crc ^ 0xFFFFFFFF).to_be_bytes());
        }

        let width = canvas.width() as usize;
        let height = canvas.height() as usize;
        let stride = width * 4;
        let pixels = canvas.pixels();

        let mut raw = Vec::with_capacity(height * (stride + 1));
        for y in 0..height {
            raw.push(0);
            raw.extend_from_slice(&pixels[y * stride..(y + 1) * stride]);
        }

        let mut out = Vec::new();
        out.extend_from_slice(&PNG_SIGNATURE);
        reference_write_chunk(&mut out, b"IHDR", &ihdr_data(canvas));
        reference_write_chunk(&mut out, b"IDAT", &reference_zlib_stored(&raw));
        reference_write_chunk(&mut out, b"IEND", &[]);
        out
    }

    fn checkerboard_canvas(width: u32, height: u32) -> Canvas {
        let mut canvas = Canvas::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let on = (x + y) % 2 == 0;
                let c = if on {
                    Color::rgba(200, 30, 90, 255)
                } else {
                    Color::rgba(10, 220, 40, 128)
                };
                canvas.fill_rect(x, y, 1, 1, c);
            }
        }
        canvas
    }

    #[test]
    fn streaming_encoder_matches_reference_for_small_canvas() {
        let canvas = checkerboard_canvas(5, 7);
        assert_eq!(encode(&canvas), reference_encode(&canvas));
    }

    #[test]
    fn streaming_encoder_matches_reference_for_1x1_canvas() {
        let canvas = checkerboard_canvas(1, 1);
        assert_eq!(encode(&canvas), reference_encode(&canvas));
    }

    #[test]
    fn streaming_encoder_matches_reference_across_stored_block_boundary() {
        // width*4 = 4004 bytes/row; 17 rows of row-data-including-filter-byte
        // straddles the 65535-byte stored-block boundary at multiple points.
        let canvas = checkerboard_canvas(1001, 17);
        assert_eq!(encode(&canvas), reference_encode(&canvas));
    }

    #[test]
    fn streaming_encoder_matches_reference_for_wide_thin_canvas() {
        let canvas = checkerboard_canvas(3000, 1);
        assert_eq!(encode(&canvas), reference_encode(&canvas));
    }

    #[test]
    fn write_to_file_matches_encode_to_vec() {
        let canvas = checkerboard_canvas(12, 9);
        let dir = std::env::temp_dir();
        let path = dir.join(format!("placard_png_write_test_{}.png", std::process::id()));
        write(&canvas, &path).expect("write should succeed");
        let from_disk = std::fs::read(&path).expect("read back should succeed");
        std::fs::remove_file(&path).ok();
        assert_eq!(from_disk, encode(&canvas));
    }
}
