use crate::error::FontError;
use crate::reader::Reader;

pub fn parse(data: &[u8], num_glyphs: u16, long_format: bool) -> Result<Vec<u32>, FontError> {
    let mut r = Reader::new(data);
    let count = num_glyphs as usize + 1;
    let mut offsets = Vec::with_capacity(count);

    if long_format {
        for _ in 0..count {
            offsets.push(r.u32()?);
        }
    } else {
        for _ in 0..count {
            offsets.push(r.u16()? as u32 * 2);
        }
    }

    Ok(offsets)
}
