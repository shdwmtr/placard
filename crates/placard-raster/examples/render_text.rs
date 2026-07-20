use placard_font::{Font, GlyfPoint, GlyphOutline};
use placard_raster::{Canvas, Color, Path, fill_path, png};

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

fn draw_text(
    canvas: &mut Canvas,
    font: &Font,
    text: &str,
    size_px: f32,
    mut x: f32,
    baseline_y: f32,
    color: Color,
) {
    let scale = size_px / font.units_per_em() as f32;
    for c in text.chars() {
        if let Some(glyph_id) = font.glyph_id_for_char(c) {
            let outline = font
                .outline(glyph_id)
                .expect("failed to read glyph outline");
            let path = glyph_path(&outline, scale, x, baseline_y);
            fill_path(canvas, &path, color);
            x += font.advance_width(glyph_id) as f32 * scale;
        }
    }
}

fn main() {
    let font_data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
        .expect("failed to read font file");
    let font = Font::parse(&font_data).expect("failed to parse font");

    let mut canvas = Canvas::new(420, 140);
    canvas.fill(Color::rgb(255, 255, 255));

    draw_text(
        &mut canvas,
        &font,
        "Hello, placard! gjy",
        32.0,
        10.0,
        60.0,
        Color::rgb(20, 20, 20),
    );
    draw_text(
        &mut canvas,
        &font,
        "AWoO 0123456789",
        24.0,
        10.0,
        110.0,
        Color::rgb(180, 30, 30),
    );

    let out_path = "target/render_text.png";
    png::write(&canvas, out_path).expect("failed to write PNG");
    println!("wrote {out_path}");
}
