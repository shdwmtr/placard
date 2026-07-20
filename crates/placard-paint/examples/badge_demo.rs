use placard_font::{Font, FontSet};
use placard_raster::{Canvas, Color, png};

fn main() {
    let font_data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
        .expect("failed to read font");
    let font = FontSet::new(Font::parse(&font_data).expect("failed to parse font"));

    let html = r#"
        <div class="badge">
            <span class="label">build</span><span class="message">passing</span>
        </div>
    "#;

    let css = r#"
        div.badge {
            margin-top: 10px;
            margin-left: 10px;
            border-radius: 4px;
        }
        span.label {
            background-color: #555555;
            color: white;
            font-size: 14px;
        }
        span.message {
            background-color: #4c1;
            color: white;
            font-size: 14px;
        }
    "#;

    let dom = placard_html::parse(html);
    let sheet = placard_css::parse(css);
    let styles = placard_style::compute(&dom, &sheet);
    let viewport_width = 200.0;
    let tree = placard_layout::build(&dom, &styles, &font, viewport_width);

    let root = tree.get(tree.root());
    let canvas_height = (root.rect.height + 20.0).ceil() as u32;
    let mut canvas = Canvas::new(viewport_width as u32, canvas_height);
    canvas.fill(Color::rgba(255, 255, 255, 255));

    placard_paint::paint(&mut canvas, &tree, &font);

    let out_path = "target/badge_demo.png";
    png::write(&canvas, out_path).expect("failed to write PNG");
    println!("wrote {out_path} ({}x{})", canvas.width(), canvas.height());
}
