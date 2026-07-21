use placard_font::{
    Font, FontFamily as FontDbFamily, FontSet, FontStyle as FontDbStyle,
    FontWeight as FontDbWeight, GlyfPoint, GlyphOutline,
};
use placard_layout::Color;
use placard_layout::{ComputedStyle, FontFamily, FontStyle, FontWeight};
use placard_raster::{Canvas, Path, fill_path};

fn contour_to_path(points: &[GlyfPoint], path: &mut Path, to_px: impl Fn(f32, f32) -> (f32, f32)) {
    let n = points.len();
    if n == 0 {
        return;
    }

    let xy = |p: &GlyfPoint| to_px(p.x as f32, p.y as f32);
    let midpoint = |a: (f32, f32), b: (f32, f32)| ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);

    let mut norm: Vec<(f32, f32, bool)> = Vec::with_capacity(n + 2);
    if points[0].on_curve {
        for p in points {
            let (x, y) = xy(p);
            norm.push((x, y, p.on_curve));
        }
    } else if points[n - 1].on_curve {
        let (x, y) = xy(&points[n - 1]);
        norm.push((x, y, true));
        for p in &points[..n - 1] {
            let (x, y) = xy(p);
            norm.push((x, y, p.on_curve));
        }
    } else {
        let start = midpoint(xy(&points[0]), xy(&points[n - 1]));
        norm.push((start.0, start.1, true));
        for p in points {
            let (x, y) = xy(p);
            norm.push((x, y, p.on_curve));
        }
    }

    let start_point = (norm[0].0, norm[0].1);
    norm.push(norm[0]);

    path.move_to(start_point.0, start_point.1);

    let mut i = 1;
    while i < norm.len() {
        let (x, y, on_curve) = norm[i];
        if on_curve {
            path.line_to(x, y);
            i += 1;
        } else {
            let ctrl = (x, y);
            let (nx, ny, n_on_curve) = norm[i + 1];
            let end = if n_on_curve {
                (nx, ny)
            } else {
                midpoint(ctrl, (nx, ny))
            };
            path.quad_to(ctrl.0, ctrl.1, end.0, end.1);
            i += if n_on_curve { 2 } else { 1 };
        }
    }
    path.close();
}

fn glyph_path(outline: &GlyphOutline, scale: f32, origin_x: f32, baseline_y: f32) -> Path {
    let mut path = Path::new();
    let to_px = |gx: f32, gy: f32| (origin_x + gx * scale, baseline_y - gy * scale);
    for contour in &outline.contours {
        contour_to_path(contour, &mut path, to_px);
    }
    path
}

pub fn resolve_font<'a>(fonts: &'a FontSet, style: &ComputedStyle) -> &'a Font {
    let families: Vec<FontDbFamily> = style
        .font_family
        .iter()
        .map(|f| match f {
            FontFamily::SansSerif => FontDbFamily::SansSerif,
            FontFamily::Serif => FontDbFamily::Serif,
            FontFamily::Monospace => FontDbFamily::Monospace,
            FontFamily::Named(name) => FontDbFamily::Named(name.clone()),
        })
        .collect();
    let weight = match style.font_weight {
        FontWeight::Normal => FontDbWeight::Normal,
        FontWeight::Bold => FontDbWeight::Bold,
    };
    let style = match style.font_style {
        FontStyle::Normal => FontDbStyle::Normal,
        FontStyle::Italic => FontDbStyle::Italic,
    };
    fonts.resolve(&families, weight, style)
}

pub fn draw_text(
    canvas: &mut Canvas,
    font: &Font,
    text: &str,
    size_px: f32,
    mut x: f32,
    baseline_y: f32,
    color: Color,
    antialias: bool,
) {
    let scale = size_px / font.units_per_em() as f32;
    let raster_color = placard_raster::Color::rgba(color.r, color.g, color.b, color.a);
    for c in text.chars() {
        if let Some(glyph_id) = font.glyph_id_for_char(c) {
            let outline = font
                .outline(glyph_id)
                .expect("failed to read glyph outline");
            let path = glyph_path(&outline, scale, x, baseline_y);
            fill_path(canvas, &path, raster_color, antialias);
            x += font.advance_width(glyph_id) as f32 * scale;
        }
    }
}

pub fn baseline_y(box_top: f32, font: &Font, size_px: f32) -> f32 {
    let scale = size_px / font.units_per_em() as f32;
    box_top + font.ascender() as f32 * scale
}
