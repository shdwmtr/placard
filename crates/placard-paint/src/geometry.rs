use placard_layout::Rect;
use placard_raster::Path;

fn clamp_corner_radius(rect: Rect, radius: f32) -> f32 {
    radius.max(0.0).min(rect.width / 2.0).min(rect.height / 2.0)
}

const COS_22_5: f32 = 0.923_879_5;

fn append_quarter_arc(path: &mut Path, cx: f32, cy: f32, r: f32, start_deg: f32, sweep_deg: f32) {
    let k = r / COS_22_5;
    let half = sweep_deg / 2.0;
    for i in 0..2 {
        let base = start_deg + i as f32 * half;
        let mid = (base + half / 2.0).to_radians();
        let end = (base + half).to_radians();
        let ctrl_x = cx + k * mid.cos();
        let ctrl_y = cy + k * mid.sin();
        let end_x = cx + r * end.cos();
        let end_y = cy + r * end.sin();
        path.quad_to(ctrl_x, ctrl_y, end_x, end_y);
    }
}

pub fn append_rounded_rect_cw(path: &mut Path, rect: Rect, radii: [f32; 4]) {
    let Rect {
        x,
        y,
        width: w,
        height: h,
    } = rect;
    let [tl, tr, br, bl] = radii.map(|r| clamp_corner_radius(rect, r));

    if tl <= 0.0 && tr <= 0.0 && br <= 0.0 && bl <= 0.0 {
        path.move_to(x, y);
        path.line_to(x + w, y);
        path.line_to(x + w, y + h);
        path.line_to(x, y + h);
        path.close();
        return;
    }

    path.move_to(x + tl, y);
    path.line_to(x + w - tr, y);
    if tr > 0.0 {
        append_quarter_arc(path, x + w - tr, y + tr, tr, 270.0, 90.0);
    }
    path.line_to(x + w, y + h - br);
    if br > 0.0 {
        append_quarter_arc(path, x + w - br, y + h - br, br, 0.0, 90.0);
    }
    path.line_to(x + bl, y + h);
    if bl > 0.0 {
        append_quarter_arc(path, x + bl, y + h - bl, bl, 90.0, 90.0);
    }
    path.line_to(x, y + tl);
    if tl > 0.0 {
        append_quarter_arc(path, x + tl, y + tl, tl, 180.0, 90.0);
    }
    path.close();
}

pub fn append_rounded_rect_ccw(path: &mut Path, rect: Rect, radii: [f32; 4]) {
    let Rect {
        x,
        y,
        width: w,
        height: h,
    } = rect;
    let [tl, tr, br, bl] = radii.map(|r| clamp_corner_radius(rect, r));

    if tl <= 0.0 && tr <= 0.0 && br <= 0.0 && bl <= 0.0 {
        path.move_to(x, y);
        path.line_to(x, y + h);
        path.line_to(x + w, y + h);
        path.line_to(x + w, y);
        path.close();
        return;
    }

    path.move_to(x + tl, y);
    if tl > 0.0 {
        append_quarter_arc(path, x + tl, y + tl, tl, 270.0, -90.0);
    }
    path.line_to(x, y + h - bl);
    if bl > 0.0 {
        append_quarter_arc(path, x + bl, y + h - bl, bl, 180.0, -90.0);
    }
    path.line_to(x + w - br, y + h);
    if br > 0.0 {
        append_quarter_arc(path, x + w - br, y + h - br, br, 90.0, -90.0);
    }
    path.line_to(x + w, y + tr);
    if tr > 0.0 {
        append_quarter_arc(path, x + w - tr, y + tr, tr, 0.0, -90.0);
    }
    path.close();
}

pub fn rounded_rect_cw(rect: Rect, radii: [f32; 4]) -> Path {
    let mut path = Path::new();
    append_rounded_rect_cw(&mut path, rect, radii);
    path
}

pub fn border_ring(
    outer_rect: Rect,
    outer_radii: [f32; 4],
    inner_rect: Rect,
    inner_radii: [f32; 4],
) -> Path {
    let mut path = Path::new();
    append_rounded_rect_cw(&mut path, outer_rect, outer_radii);
    if inner_rect.width > 0.0 && inner_rect.height > 0.0 {
        append_rounded_rect_ccw(&mut path, inner_rect, inner_radii);
    }
    path
}
