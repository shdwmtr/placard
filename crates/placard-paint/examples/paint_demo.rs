use placard_font::{Font, FontSet};
use placard_raster::{Canvas, Color, png};

fn main() {
    let font_data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
        .expect("failed to read font");
    let font = FontSet::new(Font::parse(&font_data).expect("failed to parse font"));

    let html = r#"
        <div class="card">
            <div class="header">Placard</div>
            <div class="body">
                <p class="text">A from-scratch HTML and CSS renderer, built entirely in Rust with no external libraries for parsing, layout, fonts, or rasterization.</p>
            </div>
        </div>
    "#;

    let css = r#"
        div.card {
            margin-top: 20px;
            margin-left: 20px;
            padding-top: 6px;
            padding-right: 6px;
            padding-bottom: 6px;
            padding-left: 6px;
            background-color: #f4f4f4;
            border-top-width: 3px;
            border-right-width: 3px;
            border-bottom-width: 3px;
            border-left-width: 3px;
            border-top-style: solid;
            border-right-style: solid;
            border-bottom-style: solid;
            border-left-style: solid;
            border-top-color: #2b6cb0;
            border-right-color: #2b6cb0;
            border-bottom-color: #2b6cb0;
            border-left-color: #2b6cb0;
            border-radius: 14px;
            width: 360px;
        }
        div.header {
            background-color: #2b6cb0;
            padding-top: 10px;
            padding-right: 16px;
            padding-bottom: 10px;
            padding-left: 16px;
            font-size: 22px;
            color: white;
            border-radius: 9px;
        }
        div.body {
            padding-top: 12px;
            padding-right: 16px;
            padding-bottom: 16px;
            padding-left: 16px;
        }
        p.text {
            font-size: 15px;
            color: #333333;
        }
    "#;

    let dom = placard_html::parse(html);
    let sheet = placard_css::parse(css);
    let styles = placard_style::compute(&dom, &sheet);
    let viewport_width = 420.0;
    let tree = placard_layout::build(&dom, &styles, &font, viewport_width);

    let root = tree.get(tree.root());
    let canvas_height = (root.rect.height + 20.0).ceil() as u32;
    let mut canvas = Canvas::new(viewport_width as u32, canvas_height);
    canvas.fill(Color::rgba(255, 255, 255, 255));

    placard_paint::paint(&mut canvas, &tree, &font, true);

    let out_path = "target/paint_demo.png";
    png::write(&canvas, out_path).expect("failed to write PNG");
    println!("wrote {out_path} ({}x{})", canvas.width(), canvas.height());
}
