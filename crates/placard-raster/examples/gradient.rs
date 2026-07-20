use placard_raster::{Canvas, Color, png};

fn main() {
    let width = 300;
    let height = 120;
    let mut canvas = Canvas::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let r = (255 * x / width) as u8;
            let g = (255 * y / height) as u8;
            canvas.set_pixel(x, y, Color::rgb(r, g, 128));
        }
    }

    canvas.fill_rect(20, 20, 120, 40, Color::rgba(20, 20, 20, 220));
    canvas.fill_rect(70, 45, 120, 40, Color::rgba(255, 255, 255, 160));

    let out_path = "target/gradient.png";
    png::write(&canvas, out_path).expect("failed to write PNG");
    println!("wrote {out_path}");
}
