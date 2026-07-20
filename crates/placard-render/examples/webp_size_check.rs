use placard_font::{Font, FontSet};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let html_path = &args[1];
    let width: f32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(700.0);

    let html = std::fs::read_to_string(html_path).unwrap();
    let font_data =
        std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf").unwrap();
    let fonts = FontSet::new(Font::parse(&font_data).unwrap());

    let canvas =
        placard_render::render_to_canvas(&html, Some(width), None, None, &fonts, None, None)
            .unwrap()
            .canvas;
    let png_bytes = placard_raster::png::encode(&canvas);
    let webp_bytes = placard_raster::webp::encode(&canvas).unwrap();

    println!(
        "{}x{}: raw={} png={} webp={} (webp is {:.1}% of png, {:.1}% of raw)",
        canvas.width(),
        canvas.height(),
        canvas.pixels().len(),
        png_bytes.len(),
        webp_bytes.len(),
        100.0 * webp_bytes.len() as f64 / png_bytes.len() as f64,
        100.0 * webp_bytes.len() as f64 / canvas.pixels().len() as f64,
    );

    std::fs::write("/tmp/webp_size_check_out.webp", &webp_bytes).unwrap();
    std::fs::write("/tmp/webp_size_check_out.raw", canvas.pixels()).unwrap();
}
