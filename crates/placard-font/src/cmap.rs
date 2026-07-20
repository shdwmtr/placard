use crate::error::FontError;
use crate::reader::Reader;
use std::collections::HashMap;

pub fn parse(data: &[u8]) -> HashMap<u32, u16> {
    parse_inner(data).unwrap_or_default()
}

fn parse_inner(data: &[u8]) -> Result<HashMap<u32, u16>, FontError> {
    let mut r = Reader::new(data);
    let _version = r.u16()?;
    let num_tables = r.u16()?;

    let mut best_offset: Option<u32> = None;
    let mut best_format: u16 = 0;
    let mut best_score = -1i32;

    for _ in 0..num_tables {
        let platform_id = r.u16()?;
        let encoding_id = r.u16()?;
        let offset = r.u32()?;

        let format = Reader::at(data, offset as usize).u16().unwrap_or(0);
        if format != 4 && format != 12 {
            continue;
        }
        let score = match (platform_id, encoding_id) {
            (3, 10) => 4,
            (3, 1) => 3,
            (0, _) => 2,
            (3, 0) => 1,
            _ => 0,
        };
        if score > best_score {
            best_score = score;
            best_offset = Some(offset);
            best_format = format;
        }
    }

    match best_offset {
        Some(offset) if best_format == 12 => parse_format12(data, offset as usize),
        Some(offset) => parse_format4(data, offset as usize),
        None => Ok(HashMap::new()),
    }
}

fn parse_format12(data: &[u8], offset: usize) -> Result<HashMap<u32, u16>, FontError> {
    let mut r = Reader::at(data, offset);
    let _format = r.u16()?;
    let _reserved = r.u16()?;
    let _length = r.u32()?;
    let _language = r.u32()?;
    let num_groups = r.u32()?;

    let mut map = HashMap::new();
    for _ in 0..num_groups {
        let start_char_code = r.u32()?;
        let end_char_code = r.u32()?;
        let start_glyph_id = r.u32()?;

        if end_char_code < start_char_code {
            continue;
        }
        for c in start_char_code..=end_char_code {
            let glyph_id = start_glyph_id + (c - start_char_code);
            if let Ok(glyph_id) = u16::try_from(glyph_id) {
                if glyph_id != 0 {
                    map.insert(c, glyph_id);
                }
            }
        }
    }

    Ok(map)
}

fn parse_format4(data: &[u8], offset: usize) -> Result<HashMap<u32, u16>, FontError> {
    let mut r = Reader::at(data, offset);
    let _format = r.u16()?;
    let _length = r.u16()?;
    let _language = r.u16()?;
    let seg_count = (r.u16()? / 2) as usize;
    let _search_range = r.u16()?;
    let _entry_selector = r.u16()?;
    let _range_shift = r.u16()?;

    let mut end_codes = Vec::with_capacity(seg_count);
    for _ in 0..seg_count {
        end_codes.push(r.u16()?);
    }
    let _reserved_pad = r.u16()?;

    let mut start_codes = Vec::with_capacity(seg_count);
    for _ in 0..seg_count {
        start_codes.push(r.u16()?);
    }

    let mut id_deltas = Vec::with_capacity(seg_count);
    for _ in 0..seg_count {
        id_deltas.push(r.i16()?);
    }

    let id_range_offsets_pos = r.pos();
    let mut id_range_offsets = Vec::with_capacity(seg_count);
    for _ in 0..seg_count {
        id_range_offsets.push(r.u16()?);
    }

    let mut map = HashMap::new();
    for i in 0..seg_count {
        let start = start_codes[i];
        let end = end_codes[i];
        if start == 0xFFFF && end == 0xFFFF {
            continue;
        }

        for c in start..=end {
            let glyph_id = if id_range_offsets[i] == 0 {
                c.wrapping_add(id_deltas[i] as u16)
            } else {
                let glyph_addr = id_range_offsets_pos
                    + i * 2
                    + id_range_offsets[i] as usize
                    + 2 * (c - start) as usize;
                let raw = Reader::at(data, glyph_addr).u16().unwrap_or(0);
                if raw == 0 {
                    0
                } else {
                    raw.wrapping_add(id_deltas[i] as u16)
                }
            };
            if glyph_id != 0 {
                map.insert(c as u32, glyph_id);
            }
        }
    }

    Ok(map)
}
