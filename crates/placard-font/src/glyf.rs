use crate::error::FontError;
use crate::reader::Reader;
use crate::{Font, GlyphId};

const ON_CURVE: u8 = 0x01;
const X_SHORT: u8 = 0x02;
const Y_SHORT: u8 = 0x04;
const REPEAT: u8 = 0x08;
const X_SAME_OR_POSITIVE: u8 = 0x10;
const Y_SAME_OR_POSITIVE: u8 = 0x20;

const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const ARGS_ARE_XY_VALUES: u16 = 0x0002;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;

const MAX_COMPOSITE_DEPTH: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyfPoint {
    pub x: i16,
    pub y: i16,
    pub on_curve: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GlyphOutline {
    pub contours: Vec<Vec<GlyfPoint>>,
}

pub(crate) fn outline(
    font: &Font,
    glyph_id: GlyphId,
    depth: u32,
) -> Result<GlyphOutline, FontError> {
    let idx = glyph_id.0 as usize;
    if idx + 1 >= font.loca.len() {
        return Ok(GlyphOutline::default());
    }
    let start = font.loca[idx] as usize;
    let end = font.loca[idx + 1] as usize;
    if start >= end {
        return Ok(GlyphOutline::default());
    }

    let mut r = Reader::at(&font.glyf_data, start);
    let num_contours = r.i16()?;
    r.seek(start + 10);

    if num_contours >= 0 {
        parse_simple_glyph(&mut r, num_contours as usize)
    } else if depth < MAX_COMPOSITE_DEPTH {
        parse_composite_glyph(font, &mut r, depth)
    } else {
        Ok(GlyphOutline::default())
    }
}

fn parse_simple_glyph(r: &mut Reader, num_contours: usize) -> Result<GlyphOutline, FontError> {
    let mut end_pts = Vec::with_capacity(num_contours);
    for _ in 0..num_contours {
        end_pts.push(r.u16()?);
    }
    let num_points = end_pts.last().map(|&e| e as usize + 1).unwrap_or(0);

    let instruction_length = r.u16()?;
    r.skip(instruction_length as usize);

    let mut flags = Vec::with_capacity(num_points);
    while flags.len() < num_points {
        let flag = r.u8()?;
        flags.push(flag);
        if flag & REPEAT != 0 {
            let repeat_count = r.u8()?;
            for _ in 0..repeat_count {
                flags.push(flag);
            }
        }
    }
    flags.truncate(num_points);

    let mut xs = Vec::with_capacity(num_points);
    let mut x = 0i32;
    for &flag in &flags {
        if flag & X_SHORT != 0 {
            let dx = r.u8()? as i32;
            x += if flag & X_SAME_OR_POSITIVE != 0 {
                dx
            } else {
                -dx
            };
        } else if flag & X_SAME_OR_POSITIVE == 0 {
            x += r.i16()? as i32;
        }
        xs.push(x);
    }

    let mut ys = Vec::with_capacity(num_points);
    let mut y = 0i32;
    for &flag in &flags {
        if flag & Y_SHORT != 0 {
            let dy = r.u8()? as i32;
            y += if flag & Y_SAME_OR_POSITIVE != 0 {
                dy
            } else {
                -dy
            };
        } else if flag & Y_SAME_OR_POSITIVE == 0 {
            y += r.i16()? as i32;
        }
        ys.push(y);
    }

    let mut contours = Vec::with_capacity(num_contours);
    let mut start = 0usize;
    for &end in &end_pts {
        let end = end as usize;
        let mut contour = Vec::with_capacity(end + 1 - start);
        for i in start..=end {
            contour.push(GlyfPoint {
                x: xs[i] as i16,
                y: ys[i] as i16,
                on_curve: flags[i] & ON_CURVE != 0,
            });
        }
        contours.push(contour);
        start = end + 1;
    }

    Ok(GlyphOutline { contours })
}

fn parse_composite_glyph(
    font: &Font,
    r: &mut Reader,
    depth: u32,
) -> Result<GlyphOutline, FontError> {
    let mut result = GlyphOutline::default();

    loop {
        let flags = r.u16()?;
        let glyph_index = r.u16()?;

        let (dx, dy) = if flags & ARG_1_AND_2_ARE_WORDS != 0 {
            (r.i16()? as i32, r.i16()? as i32)
        } else {
            (r.i8()? as i32, r.i8()? as i32)
        };
        let (dx, dy) = if flags & ARGS_ARE_XY_VALUES != 0 {
            (dx, dy)
        } else {
            (0, 0)
        };

        if flags & WE_HAVE_A_SCALE != 0 {
            r.skip(2);
        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            r.skip(4);
        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
            r.skip(8);
        }

        if let Ok(sub) = outline(font, GlyphId(glyph_index), depth + 1) {
            for contour in sub.contours {
                let translated = contour
                    .into_iter()
                    .map(|p| GlyfPoint {
                        x: p.x.saturating_add(dx as i16),
                        y: p.y.saturating_add(dy as i16),
                        on_curve: p.on_curve,
                    })
                    .collect();
                result.contours.push(translated);
            }
        }

        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }

    Ok(result)
}
