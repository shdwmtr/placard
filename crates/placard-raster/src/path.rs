use crate::canvas::{Canvas, Color};

#[derive(Debug, Clone, Copy)]
enum Verb {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    Close,
}

#[derive(Debug, Clone, Default)]
pub struct Path {
    verbs: Vec<Verb>,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.verbs.push(Verb::MoveTo(x, y));
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.verbs.push(Verb::LineTo(x, y));
    }

    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.verbs.push(Verb::QuadTo(cx, cy, x, y));
    }

    pub fn close(&mut self) {
        self.verbs.push(Verb::Close);
    }

    fn flatten(&self) -> Vec<Vec<(f32, f32)>> {
        const QUAD_SEGMENTS: usize = 8;

        let mut subpaths = Vec::new();
        let mut current: Vec<(f32, f32)> = Vec::new();
        let mut start = (0.0, 0.0);
        let mut pos = (0.0, 0.0);

        for verb in &self.verbs {
            match *verb {
                Verb::MoveTo(x, y) => {
                    if current.len() > 1 {
                        subpaths.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                    start = (x, y);
                    pos = start;
                    current.push(pos);
                }
                Verb::LineTo(x, y) => {
                    pos = (x, y);
                    current.push(pos);
                }
                Verb::QuadTo(cx, cy, x, y) => {
                    for i in 1..=QUAD_SEGMENTS {
                        let t = i as f32 / QUAD_SEGMENTS as f32;
                        let mt = 1.0 - t;
                        let px = mt * mt * pos.0 + 2.0 * mt * t * cx + t * t * x;
                        let py = mt * mt * pos.1 + 2.0 * mt * t * cy + t * t * y;
                        current.push((px, py));
                    }
                    pos = (x, y);
                }
                Verb::Close => {
                    if pos != start {
                        current.push(start);
                    }
                    pos = start;
                }
            }
        }
        if current.len() > 1 {
            subpaths.push(current);
        }
        subpaths
    }
}

/// Fills `path` on `canvas`. The accumulator and per-row scan below only
/// cover `path`'s bounding box (clamped to the canvas and padded by a pixel
/// for anti-aliasing), not the whole canvas -- filling one small glyph on a
/// large canvas costs work proportional to the glyph, not to the canvas.
pub fn fill_path(canvas: &mut Canvas, path: &Path, color: Color, antialias: bool) {
    let canvas_width_total = canvas.width() as usize;
    let canvas_height_total = canvas.height() as usize;
    if canvas_width_total == 0 || canvas_height_total == 0 {
        return;
    }

    let subpaths = path.flatten();
    if subpaths.is_empty() {
        return;
    }

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for subpath in &subpaths {
        for &(x, y) in subpath {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    let clip_x0 = (min_x.floor() as i64 - 1).clamp(0, canvas_width_total as i64) as usize;
    let clip_x1 = (max_x.ceil() as i64 + 1).clamp(0, canvas_width_total as i64) as usize;
    let clip_y0 = (min_y.floor() as i64 - 1).clamp(0, canvas_height_total as i64) as usize;
    let clip_y1 = (max_y.ceil() as i64 + 1).clamp(0, canvas_height_total as i64) as usize;
    if clip_x1 <= clip_x0 || clip_y1 <= clip_y0 {
        return;
    }

    let width = clip_x1 - clip_x0;
    let height = clip_y1 - clip_y0;
    let offset_x = clip_x0 as f32;
    let offset_y = clip_y0 as f32;
    let local = |p: (f32, f32)| (p.0 - offset_x, p.1 - offset_y);

    let accum_width = width + 2;
    let mut accum = vec![0.0f32; accum_width * height];
    let canvas_width = width as f32;

    for subpath in &subpaths {
        for w in subpath.windows(2) {
            add_edge(
                &mut accum,
                accum_width,
                height,
                canvas_width,
                local(w[0]),
                local(w[1]),
            );
        }
        if let (Some(&first), Some(&last)) = (subpath.first(), subpath.last()) {
            if first != last {
                add_edge(
                    &mut accum,
                    accum_width,
                    height,
                    canvas_width,
                    local(last),
                    local(first),
                );
            }
        }
    }

    for row in 0..height {
        let row_offset = row * accum_width;
        let mut cover = 0.0f32;
        for col in 0..width {
            cover += accum[row_offset + col];
            let mut alpha = cover.abs().min(1.0);
            if !antialias {
                alpha = if alpha >= 0.5 { 1.0 } else { 0.0 };
            }
            if alpha > 1.0 / 512.0 {
                let mut c = color;
                c.a = (c.a as f32 * alpha).round() as u8;
                canvas.blend_pixel((col + clip_x0) as u32, (row + clip_y0) as u32, c);
            }
        }
    }
}

fn add_edge(
    accum: &mut [f32],
    accum_width: usize,
    height: usize,
    canvas_width: f32,
    p0: (f32, f32),
    p1: (f32, f32),
) {
    if p0.1 == p1.1 {
        return;
    }
    let ((x0, y0), (x1, y1), dir) = if p0.1 < p1.1 {
        (p0, p1, 1.0)
    } else {
        (p1, p0, -1.0)
    };

    let y_start = y0.max(0.0);
    let y_end = y1.min(height as f32);
    if y_start >= y_end {
        return;
    }

    let dxdy = (x1 - x0) / (y1 - y0);
    let mut y_cur = y_start;
    let mut x_cur = x0 + dxdy * (y_start - y0);

    while y_cur < y_end {
        let row = y_cur.floor();
        let row_bottom = (row + 1.0).min(y_end);
        let x_next = x0 + dxdy * (row_bottom - y0);
        add_row_segment(
            accum,
            accum_width,
            row as i32,
            canvas_width,
            x_cur,
            y_cur,
            x_next,
            row_bottom,
            dir,
        );
        x_cur = x_next;
        y_cur = row_bottom;
    }
}

fn add_row_segment(
    accum: &mut [f32],
    accum_width: usize,
    row: i32,
    canvas_width: f32,
    xa: f32,
    ya: f32,
    xb: f32,
    yb: f32,
    dir: f32,
) {
    let row_offset = row as usize * accum_width;
    let d_total = (yb - ya) * dir;
    if d_total == 0.0 {
        return;
    }

    // At most 0.0, 1.0, and the two canvas-edge clip breakpoints -- a fixed
    // stack array avoids a heap allocation on every scanline segment (this
    // runs per edge per row, so for text-heavy renders that's thousands of
    // tiny allocations otherwise).
    let mut breakpoints = [0.0f32, 1.0, 0.0, 0.0];
    let mut count = 2usize;
    if xb != xa {
        let t0 = (0.0 - xa) / (xb - xa);
        let t1 = (canvas_width - xa) / (xb - xa);
        for t in [t0, t1] {
            if t > 0.0 && t < 1.0 {
                breakpoints[count] = t;
                count += 1;
            }
        }
    }
    let breakpoints = &mut breakpoints[..count];
    breakpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for w in breakpoints.windows(2) {
        let (t0, t1) = (w[0], w[1]);
        if t1 <= t0 {
            continue;
        }
        let seg_xa = xa + t0 * (xb - xa);
        let seg_xb = xa + t1 * (xb - xa);
        let seg_d = d_total * (t1 - t0);

        let mid_x = 0.5 * (seg_xa + seg_xb);
        if mid_x < 0.0 {
            accum[row_offset] += seg_d;
        } else if mid_x > canvas_width {
        } else {
            add_in_bounds_segment(accum, row_offset, seg_xa, seg_xb, seg_d);
        }
    }
}

fn add_in_bounds_segment(accum: &mut [f32], row_offset: usize, xa: f32, xb: f32, d_total: f32) {
    let total_dx = xb - xa;

    if total_dx == 0.0 {
        let col = xa.floor();
        let col_i = col as i32;
        let xbar = xa - col;
        accum[row_offset + col_i as usize] += d_total * (1.0 - xbar);
        accum[row_offset + col_i as usize + 1] += d_total * xbar;
        return;
    }

    let x_dir = if total_dx > 0.0 { 1.0 } else { -1.0 };
    let mut x_cur = xa;

    loop {
        let col = if x_dir > 0.0 {
            x_cur.floor()
        } else {
            x_cur.ceil() - 1.0
        };
        let col_i = col as i32;
        let col_boundary = if x_dir > 0.0 { col + 1.0 } else { col };
        let reached_end = if x_dir > 0.0 {
            xb <= col_boundary
        } else {
            xb >= col_boundary
        };
        let x_next = if reached_end { xb } else { col_boundary };

        let frac = (x_next - x_cur) / total_dx;
        let d = d_total * frac;
        let xbar = 0.5 * (x_cur + x_next) - col;

        accum[row_offset + col_i as usize] += d * (1.0 - xbar);
        accum[row_offset + col_i as usize + 1] += d * xbar;

        if reached_end {
            break;
        }
        x_cur = x_next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(canvas: &Canvas, x: u32, y: u32) -> u8 {
        canvas.get_pixel(x, y).a
    }

    #[test]
    fn axis_aligned_rectangle_fully_inside() {
        let mut canvas = Canvas::new(8, 8);
        let mut path = Path::new();
        path.move_to(2.0, 2.0);
        path.line_to(5.0, 2.0);
        path.line_to(5.0, 5.0);
        path.line_to(2.0, 5.0);
        path.close();

        fill_path(&mut canvas, &path, Color::rgba(0, 0, 0, 255), true);

        assert_eq!(alpha_at(&canvas, 3, 3), 255);
        assert_eq!(alpha_at(&canvas, 0, 0), 0);
        assert_eq!(alpha_at(&canvas, 6, 6), 0);
        assert_eq!(alpha_at(&canvas, 2, 2), 255);
        assert_eq!(alpha_at(&canvas, 4, 4), 255);
        assert_eq!(alpha_at(&canvas, 5, 5), 0);
    }

    #[test]
    fn fractional_edge_gives_partial_coverage() {
        let mut canvas = Canvas::new(8, 8);
        let mut path = Path::new();
        path.move_to(2.5, 2.0);
        path.line_to(5.5, 2.0);
        path.line_to(5.5, 3.0);
        path.line_to(2.5, 3.0);
        path.close();

        fill_path(&mut canvas, &path, Color::rgba(0, 0, 0, 255), true);

        assert_eq!(alpha_at(&canvas, 3, 2), 255);
        assert_eq!(alpha_at(&canvas, 4, 2), 255);
        let left_edge = alpha_at(&canvas, 2, 2);
        let right_edge = alpha_at(&canvas, 5, 2);
        assert!(
            (110..=145).contains(&left_edge),
            "left edge alpha was {left_edge}"
        );
        assert!(
            (110..=145).contains(&right_edge),
            "right edge alpha was {right_edge}"
        );
    }

    #[test]
    fn shape_extending_off_canvas_left_fills_visible_columns() {
        let mut canvas = Canvas::new(8, 8);
        let mut path = Path::new();
        path.move_to(-2.0, 2.0);
        path.line_to(3.0, 2.0);
        path.line_to(3.0, 5.0);
        path.line_to(-2.0, 5.0);
        path.close();

        fill_path(&mut canvas, &path, Color::rgba(0, 0, 0, 255), true);

        assert_eq!(alpha_at(&canvas, 0, 3), 255);
        assert_eq!(alpha_at(&canvas, 1, 3), 255);
        assert_eq!(alpha_at(&canvas, 2, 3), 255);
        assert_eq!(alpha_at(&canvas, 3, 3), 0);
    }

    #[test]
    fn shape_extending_off_canvas_right_fills_visible_columns() {
        let mut canvas = Canvas::new(8, 8);
        let mut path = Path::new();
        path.move_to(5.0, 2.0);
        path.line_to(20.0, 2.0);
        path.line_to(20.0, 5.0);
        path.line_to(5.0, 5.0);
        path.close();

        fill_path(&mut canvas, &path, Color::rgba(0, 0, 0, 255), true);

        assert_eq!(alpha_at(&canvas, 4, 3), 0);
        assert_eq!(alpha_at(&canvas, 5, 3), 255);
        assert_eq!(alpha_at(&canvas, 7, 3), 255);
    }

    #[test]
    fn hole_via_opposite_winding_is_transparent() {
        let mut canvas = Canvas::new(10, 10);
        let mut path = Path::new();
        path.move_to(1.0, 1.0);
        path.line_to(9.0, 1.0);
        path.line_to(9.0, 9.0);
        path.line_to(1.0, 9.0);
        path.close();

        path.move_to(3.0, 3.0);
        path.line_to(3.0, 7.0);
        path.line_to(7.0, 7.0);
        path.line_to(7.0, 3.0);
        path.close();

        fill_path(&mut canvas, &path, Color::rgba(0, 0, 0, 255), true);

        assert_eq!(alpha_at(&canvas, 2, 2), 255);
        assert_eq!(alpha_at(&canvas, 5, 5), 0);
    }

    #[test]
    fn quad_curve_bulges_beyond_chord() {
        let mut canvas = Canvas::new(20, 20);
        let mut path = Path::new();
        path.move_to(2.0, 10.0);
        path.quad_to(10.0, 2.0, 18.0, 10.0);
        path.line_to(18.0, 12.0);
        path.line_to(2.0, 12.0);
        path.close();

        fill_path(&mut canvas, &path, Color::rgba(0, 0, 0, 255), true);

        assert_eq!(alpha_at(&canvas, 10, 11), 255);
        assert_eq!(alpha_at(&canvas, 10, 4), 0);
        assert_eq!(alpha_at(&canvas, 10, 7), 255);
    }
}
