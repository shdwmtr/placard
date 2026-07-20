use placard_raster::{Canvas, Color};

fn checkerboard(width: u32, height: u32) -> Canvas {
    let mut canvas = Canvas::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let on = (x + y) % 2 == 0;
            let c = if on {
                Color::rgba(200, 30, 90, 255)
            } else {
                Color::rgba(10, 220, 40, 255)
            };
            canvas.fill_rect(x, y, 1, 1, c);
        }
    }
    canvas
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kind = args.get(1).map(String::as_str).unwrap_or("solid");
    let out_path = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("/tmp/webp_check.webp");

    let canvas = match kind {
        "solid1" => {
            let mut c = Canvas::new(1, 1);
            c.fill(Color::rgba(123, 45, 67, 255));
            c
        }
        "solid" => {
            let mut c = Canvas::new(50, 50);
            c.fill(Color::rgba(11, 22, 33, 255));
            c
        }
        "checker" => checkerboard(37, 23),
        other => panic!("unknown kind: {other}"),
    };

    placard_raster::webp::write(&canvas, out_path).expect("encode should succeed");
    println!(
        "wrote {out_path} ({}x{}, {} bytes raw)",
        canvas.width(),
        canvas.height(),
        canvas.pixels().len()
    );

    // Dump raw pixels alongside so the shell script can diff them directly.
    let raw_path = format!("{out_path}.raw");
    std::fs::write(&raw_path, canvas.pixels()).expect("raw dump should succeed");
    println!("wrote {raw_path}");
}
