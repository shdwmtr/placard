use placard_font::Font;

fn main() {
    let path = "/usr/share/fonts/liberation/LiberationSans-Regular.ttf";
    let data = std::fs::read(path).expect("failed to read font file");
    let font = Font::parse(&data).expect("failed to parse font");

    println!("units_per_em: {}", font.units_per_em());
    println!("ascender: {}", font.ascender());
    println!("descender: {}", font.descender());
    println!("line_gap: {}", font.line_gap());
    println!("num_glyphs: {}", font.num_glyphs());
    println!();

    for c in ['A', 'g', 'o', '.', 'W', ' '] {
        let Some(glyph_id) = font.glyph_id_for_char(c) else {
            println!("{c:?}: no glyph mapping");
            continue;
        };
        let advance = font.advance_width(glyph_id);
        let outline = font.outline(glyph_id).expect("failed to read outline");
        let contour_count = outline.contours.len();
        let point_count: usize = outline.contours.iter().map(|c| c.len()).sum();

        println!(
            "{c:?}: glyph_id={} advance_width={} contours={} points={}",
            glyph_id.0, advance, contour_count, point_count
        );
    }
}
