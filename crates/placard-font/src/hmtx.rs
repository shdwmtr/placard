use crate::error::FontError;
use crate::reader::Reader;

pub fn parse(
    data: &[u8],
    num_glyphs: u16,
    num_h_metrics: u16,
) -> Result<Vec<(u16, i16)>, FontError> {
    let mut r = Reader::new(data);
    let mut metrics = Vec::with_capacity(num_glyphs as usize);

    let mut last_advance = 0u16;
    for _ in 0..num_h_metrics {
        let advance = r.u16()?;
        let lsb = r.i16()?;
        last_advance = advance;
        metrics.push((advance, lsb));
    }

    let remaining = num_glyphs.saturating_sub(num_h_metrics);
    for _ in 0..remaining {
        let lsb = r.i16()?;
        metrics.push((last_advance, lsb));
    }

    Ok(metrics)
}
