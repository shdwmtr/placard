mod cmap;
mod error;
mod fontset;
mod glyf;
mod hmtx;
mod loca;
mod reader;
mod tables;

use reader::Reader;
use std::collections::HashMap;

pub use error::FontError;
pub use fontset::{FontFamily, FontSet, FontStyle, FontWeight};
pub use glyf::{GlyfPoint, GlyphOutline};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u16);

const SFNT_VERSION_TRUETYPE: u32 = 0x0001_0000;
const SFNT_VERSION_TRUE: u32 = 0x7472_7565;
const SFNT_VERSION_OTTO: u32 = 0x4F54_544F;

pub struct Font {
    units_per_em: u16,
    ascender: i16,
    descender: i16,
    line_gap: i16,
    num_glyphs: u16,
    loca: Vec<u32>,
    glyf_data: Vec<u8>,
    hmtx: Vec<(u16, i16)>,
    cmap: HashMap<u32, u16>,
    family_name: Option<String>,
}

impl Font {
    pub fn parse(data: &[u8]) -> Result<Font, FontError> {
        let mut r = Reader::new(data);
        let sfnt_version = r.u32()?;
        if sfnt_version == SFNT_VERSION_OTTO
            || (sfnt_version != SFNT_VERSION_TRUETYPE && sfnt_version != SFNT_VERSION_TRUE)
        {
            return Err(FontError::UnsupportedOutlineFormat);
        }

        let num_tables = r.u16()?;
        r.skip(6);

        let mut tables: HashMap<[u8; 4], (u32, u32)> = HashMap::new();
        for _ in 0..num_tables {
            let tag = r.tag()?;
            r.skip(4);
            let offset = r.u32()?;
            let length = r.u32()?;
            tables.insert(tag, (offset, length));
        }

        let table_bytes = |tag: &[u8; 4], name: &'static str| -> Result<&[u8], FontError> {
            let &(offset, length) = tables.get(tag).ok_or(FontError::MissingTable(name))?;
            let start = offset as usize;
            let end = start + length as usize;
            data.get(start..end).ok_or(FontError::UnexpectedEof)
        };

        let head = tables::parse_head(table_bytes(b"head", "head")?)?;
        let hhea = tables::parse_hhea(table_bytes(b"hhea", "hhea")?)?;
        let num_glyphs = tables::parse_maxp_num_glyphs(table_bytes(b"maxp", "maxp")?)?;

        let loca = loca::parse(
            table_bytes(b"loca", "loca")?,
            num_glyphs,
            head.index_to_loc_format == 1,
        )?;
        let glyf_data = table_bytes(b"glyf", "glyf")?.to_vec();
        let hmtx = hmtx::parse(
            table_bytes(b"hmtx", "hmtx")?,
            num_glyphs,
            hhea.num_h_metrics,
        )?;

        let cmap = tables
            .get(b"cmap")
            .and_then(|&(offset, length)| data.get(offset as usize..(offset + length) as usize))
            .map(cmap::parse)
            .unwrap_or_default();

        let family_name = tables
            .get(b"name")
            .and_then(|&(offset, length)| data.get(offset as usize..(offset + length) as usize))
            .and_then(tables::parse_name);

        Ok(Font {
            units_per_em: head.units_per_em,
            ascender: hhea.ascender,
            descender: hhea.descender,
            line_gap: hhea.line_gap,
            num_glyphs,
            loca,
            glyf_data,
            hmtx,
            cmap,
            family_name,
        })
    }

    pub fn family_name(&self) -> Option<&str> {
        self.family_name.as_deref()
    }

    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    pub fn ascender(&self) -> i16 {
        self.ascender
    }

    pub fn descender(&self) -> i16 {
        self.descender
    }

    pub fn line_gap(&self) -> i16 {
        self.line_gap
    }

    pub fn num_glyphs(&self) -> u16 {
        self.num_glyphs
    }

    pub fn glyph_id_for_char(&self, c: char) -> Option<GlyphId> {
        self.cmap.get(&(c as u32)).copied().map(GlyphId)
    }

    pub fn advance_width(&self, glyph_id: GlyphId) -> u16 {
        self.hmtx
            .get(glyph_id.0 as usize)
            .map(|&(advance, _)| advance)
            .unwrap_or(0)
    }

    pub fn outline(&self, glyph_id: GlyphId) -> Result<GlyphOutline, FontError> {
        glyf::outline(self, glyph_id, 0)
    }
}
