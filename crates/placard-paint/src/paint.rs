use crate::geometry::{border_ring, rounded_rect_cw};
use crate::text::{baseline_y, draw_text, resolve_font};
use placard_font::FontSet;
use placard_layout::{BorderStyle, BoxKind, BoxNode, Color, LayoutNodeId, LayoutTree, Rect};
use placard_raster::{Canvas, fill_path};

fn to_raster_color(c: Color) -> placard_raster::Color {
    placard_raster::Color::rgba(c.r, c.g, c.b, c.a)
}

fn inset_rect(rect: Rect, top: f32, right: f32, bottom: f32, left: f32) -> Rect {
    Rect {
        x: rect.x + left,
        y: rect.y + top,
        width: (rect.width - left - right).max(0.0),
        height: (rect.height - top - bottom).max(0.0),
    }
}

fn effective_border_widths(node: &BoxNode) -> [f32; 4] {
    let mut widths = node.style.border_width;
    for i in 0..4 {
        if node.style.border_style[i] != BorderStyle::Solid {
            widths[i] = 0.0;
        }
    }
    widths
}

fn paint_background_and_border(canvas: &mut Canvas, node: &BoxNode, antialias: bool) {
    let radius = node.style.border_radius;
    let widths = effective_border_widths(node);
    let has_border = widths.iter().any(|&w| w > 0.0);

    if node.style.background_color.a > 0 {
        if radius.iter().all(|&r| r == 0.0) {
            // Square corners on an axis-aligned rect never need
            // antialiasing; going through the coverage rasterizer here
            // would blend the fractional remainder of the box's edge
            // (sub-pixel layout positions are routine) into a faint,
            // wrong-looking sliver of background color instead of a crisp
            // edge. Snapping straight to whole pixels matches how the
            // border fill below already handles the no-radius case.
            let r = node.rect;
            let x0 = r.x.round();
            let y0 = r.y.round();
            let x1 = (r.x + r.width).round();
            let y1 = (r.y + r.height).round();
            canvas.fill_rect(
                x0.max(0.0) as u32,
                y0.max(0.0) as u32,
                (x1 - x0).max(0.0) as u32,
                (y1 - y0).max(0.0) as u32,
                to_raster_color(node.style.background_color),
            );
        } else {
            let path = rounded_rect_cw(node.rect, radius);
            fill_path(
                canvas,
                &path,
                to_raster_color(node.style.background_color),
                antialias,
            );
        }
    }

    if !has_border {
        return;
    }

    let uniform_color = node.style.border_color[1..]
        .iter()
        .all(|c| *c == node.style.border_color[0]);

    if radius.iter().any(|&r| r > 0.0) || uniform_color {
        let inner_rect = inset_rect(node.rect, widths[0], widths[1], widths[2], widths[3]);
        let inner_radius = [
            (radius[0] - widths[0].max(widths[3])).max(0.0),
            (radius[1] - widths[0].max(widths[1])).max(0.0),
            (radius[2] - widths[2].max(widths[1])).max(0.0),
            (radius[3] - widths[2].max(widths[3])).max(0.0),
        ];
        let ring = border_ring(node.rect, radius, inner_rect, inner_radius);
        fill_path(
            canvas,
            &ring,
            to_raster_color(node.style.border_color[0]),
            antialias,
        );
    } else {
        let r = node.rect;
        if widths[0] > 0.0 {
            canvas.fill_rect(
                r.x as u32,
                r.y as u32,
                r.width as u32,
                widths[0] as u32,
                to_raster_color(node.style.border_color[0]),
            );
        }
        if widths[2] > 0.0 {
            canvas.fill_rect(
                r.x as u32,
                (r.y + r.height - widths[2]) as u32,
                r.width as u32,
                widths[2] as u32,
                to_raster_color(node.style.border_color[2]),
            );
        }
        if widths[3] > 0.0 {
            canvas.fill_rect(
                r.x as u32,
                r.y as u32,
                widths[3] as u32,
                r.height as u32,
                to_raster_color(node.style.border_color[3]),
            );
        }
        if widths[1] > 0.0 {
            canvas.fill_rect(
                (r.x + r.width - widths[1]) as u32,
                r.y as u32,
                widths[1] as u32,
                r.height as u32,
                to_raster_color(node.style.border_color[1]),
            );
        }
    }
}

fn paint_box(
    canvas: &mut Canvas,
    tree: &LayoutTree,
    id: LayoutNodeId,
    fonts: &FontSet,
    antialias: bool,
) {
    let node = tree.get(id);
    match &node.kind {
        BoxKind::Block | BoxKind::InlineBackground => {
            paint_background_and_border(canvas, node, antialias);
        }
        BoxKind::Text { content } => {
            let font = resolve_font(fonts, &node.style);
            let baseline = baseline_y(node.rect.y, font, node.style.font_size);
            draw_text(
                canvas,
                font,
                content,
                node.style.font_size,
                node.rect.x,
                baseline,
                node.style.color,
                node.style.letter_spacing,
                antialias,
            );
        }
    }
    for &child in tree.children(id) {
        paint_box(canvas, tree, child, fonts, antialias);
    }
}

pub fn paint(canvas: &mut Canvas, tree: &LayoutTree, fonts: &FontSet, antialias: bool) {
    paint_box(canvas, tree, tree.root(), fonts, antialias);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_fonts() -> FontSet {
        let data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
            .expect("failed to read font");
        FontSet::new(placard_font::Font::parse(&data).expect("failed to parse font"))
    }

    #[test]
    fn higher_z_index_paints_on_top_regardless_of_document_order() {
        let fonts = test_fonts();
        let dom = placard_html::parse(
            "<div class=\"wrap\">
                <div class=\"front\"></div>
                <div class=\"back\"></div>
             </div>",
        );
        let sheet = placard_css::parse(
            "div.wrap { position: relative; width: 100px; height: 100px; }
             div.front, div.back {
                position: absolute; top: 0; left: 0; width: 50px; height: 50px;
             }
             div.front { z-index: 1; background-color: blue; }
             div.back { z-index: 5; background-color: red; }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        let tree = placard_layout::build(&dom, &styles, &fonts, 200.0);

        let mut canvas = Canvas::new(200, 100);
        paint(&mut canvas, &tree, &fonts, true);

        let pixel = canvas.get_pixel(10, 10);
        assert_eq!(pixel, placard_raster::Color::rgba(255, 0, 0, 255));
    }

    #[test]
    fn zero_radius_background_snaps_to_whole_pixels_at_fractional_edges() {
        let fonts = test_fonts();
        let dom = placard_html::parse("<div class=\"box\"></div>");
        let sheet = placard_css::parse(
            "div.box { width: 50%; height: 20px; background-color: rgb(0, 128, 0); }",
        );
        let styles = placard_style::compute(&dom, &sheet);
        // 50% of a 155px container lands the box's right edge at x=77.5 --
        // a fractional pixel boundary that used to blend into a faint,
        // partially-covered sliver instead of a crisp edge.
        let tree = placard_layout::build(&dom, &styles, &fonts, 155.0);

        let mut canvas = Canvas::new(155, 20);
        canvas.fill(placard_raster::Color::rgba(255, 255, 255, 255));
        paint(&mut canvas, &tree, &fonts, true);

        assert_eq!(
            canvas.get_pixel(77, 10),
            placard_raster::Color::rgba(0, 128, 0, 255)
        );
        assert_eq!(
            canvas.get_pixel(78, 10),
            placard_raster::Color::rgba(255, 255, 255, 255)
        );
    }
}
