use crate::error::FontError;
use crate::reader::Reader;

pub struct HeadTable {
    pub units_per_em: u16,
    pub index_to_loc_format: i16,
}

pub fn parse_head(data: &[u8]) -> Result<HeadTable, FontError> {
    let mut r = Reader::new(data);
    r.seek(18);
    let units_per_em = r.u16()?;
    r.seek(50);
    let index_to_loc_format = r.i16()?;
    Ok(HeadTable {
        units_per_em,
        index_to_loc_format,
    })
}

pub struct HheaTable {
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub num_h_metrics: u16,
}

pub fn parse_hhea(data: &[u8]) -> Result<HheaTable, FontError> {
    let mut r = Reader::new(data);
    r.seek(4);
    let ascender = r.i16()?;
    let descender = r.i16()?;
    let line_gap = r.i16()?;
    r.seek(34);
    let num_h_metrics = r.u16()?;
    Ok(HheaTable {
        ascender,
        descender,
        line_gap,
        num_h_metrics,
    })
}

pub fn parse_maxp_num_glyphs(data: &[u8]) -> Result<u16, FontError> {
    let mut r = Reader::new(data);
    r.seek(4);
    r.u16()
}

pub fn parse_name(data: &[u8]) -> Option<String> {
    let mut r = Reader::new(data);
    r.skip(2); // format
    let count = r.u16().ok()?;
    let string_storage = r.u16().ok()? as usize;

    struct Candidate {
        name_id: u16,
        is_unicode: bool,
        text: String,
    }
    let mut best: Option<Candidate> = None;

    for _ in 0..count {
        let platform_id = r.u16().ok()?;
        let encoding_id = r.u16().ok()?;
        r.skip(2); // language_id
        let name_id = r.u16().ok()?;
        let length = r.u16().ok()? as usize;
        let str_offset = r.u16().ok()? as usize;

        if name_id != 1 && name_id != 16 {
            continue;
        }
        let start = string_storage.checked_add(str_offset)?;
        let bytes = data.get(start..start.checked_add(length)?)?;
        let is_unicode = platform_id == 3 && matches!(encoding_id, 0 | 1 | 10);
        let text = if is_unicode {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&units).ok()?
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        };
        if text.is_empty() {
            continue;
        }

        let better = match &best {
            None => true,
            Some(current) => {
                (name_id == 16 && current.name_id != 16)
                    || (name_id == current.name_id && is_unicode && !current.is_unicode)
            }
        };
        if better {
            best = Some(Candidate {
                name_id,
                is_unicode,
                text,
            });
        }
    }

    best.map(|c| c.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_name_table(records: &[(u16, u16, u16, u16, &str)]) -> Vec<u8> {
        let mut strings = Vec::new();
        let mut entries = Vec::new();
        for &(platform_id, encoding_id, language_id, name_id, text) in records {
            let bytes = if platform_id == 3 {
                text.encode_utf16()
                    .flat_map(|u| u.to_be_bytes())
                    .collect::<Vec<u8>>()
            } else {
                text.as_bytes().to_vec()
            };
            entries.push((
                platform_id,
                encoding_id,
                language_id,
                name_id,
                bytes.len() as u16,
                strings.len() as u16,
            ));
            strings.extend_from_slice(&bytes);
        }

        let mut out = Vec::new();
        out.extend_from_slice(&0u16.to_be_bytes()); // format
        out.extend_from_slice(&(records.len() as u16).to_be_bytes()); // count
        let string_offset = 6 + records.len() as u16 * 12;
        out.extend_from_slice(&string_offset.to_be_bytes());
        for (platform_id, encoding_id, language_id, name_id, length, offset) in entries {
            out.extend_from_slice(&platform_id.to_be_bytes());
            out.extend_from_slice(&encoding_id.to_be_bytes());
            out.extend_from_slice(&language_id.to_be_bytes());
            out.extend_from_slice(&name_id.to_be_bytes());
            out.extend_from_slice(&length.to_be_bytes());
            out.extend_from_slice(&offset.to_be_bytes());
        }
        out.extend_from_slice(&strings);
        out
    }

    #[test]
    fn prefers_typographic_family_over_legacy_family() {
        let table = build_name_table(&[
            (3, 1, 0x0409, 1, "Geist ExtraBold"),
            (3, 1, 0x0409, 16, "Geist"),
        ]);
        assert_eq!(parse_name(&table).as_deref(), Some("Geist"));
    }

    #[test]
    fn falls_back_to_legacy_family_when_no_typographic_name() {
        let table = build_name_table(&[(3, 1, 0x0409, 1, "Inter")]);
        assert_eq!(parse_name(&table).as_deref(), Some("Inter"));
    }

    #[test]
    fn prefers_windows_unicode_record_over_mac_record() {
        let table = build_name_table(&[
            (1, 0, 0, 1, "JetBrainsMono NFM"),
            (3, 1, 0x0409, 1, "JetBrainsMono Nerd Font Mono"),
        ]);
        assert_eq!(
            parse_name(&table).as_deref(),
            Some("JetBrainsMono Nerd Font Mono")
        );
    }

    #[test]
    fn falls_back_to_mac_record_when_no_windows_record() {
        let table = build_name_table(&[(1, 0, 0, 1, "Geist")]);
        assert_eq!(parse_name(&table).as_deref(), Some("Geist"));
    }

    #[test]
    fn returns_none_for_a_name_table_with_no_family_records() {
        let table = build_name_table(&[(3, 1, 0x0409, 2, "Bold")]);
        assert_eq!(parse_name(&table), None);
    }
}
