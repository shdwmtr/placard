#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * 4
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        let i = self.index(x, y);
        Color::rgba(
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        )
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = self.index(x, y);
        self.pixels[i] = color.r;
        self.pixels[i + 1] = color.g;
        self.pixels[i + 2] = color.b;
        self.pixels[i + 3] = color.a;
    }

    pub fn blend_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height || color.a == 0 {
            return;
        }
        if color.a == 255 {
            self.set_pixel(x, y, color);
            return;
        }

        let dst = self.get_pixel(x, y);
        let sa = color.a as f32 / 255.0;
        let da = dst.a as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);

        let blend_channel = |sc: u8, dc: u8| -> u8 {
            if out_a <= 0.0 {
                return 0;
            }
            let sc = sc as f32 / 255.0;
            let dc = dc as f32 / 255.0;
            let out_c = (sc * sa + dc * da * (1.0 - sa)) / out_a;
            (out_c * 255.0).round().clamp(0.0, 255.0) as u8
        };

        let out = Color::rgba(
            blend_channel(color.r, dst.r),
            blend_channel(color.g, dst.g),
            blend_channel(color.b, dst.b),
            (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
        );
        self.set_pixel(x, y, out);
    }

    pub fn fill(&mut self, color: Color) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel[0] = color.r;
            pixel[1] = color.g;
            pixel[2] = color.b;
            pixel[3] = color.a;
        }
    }

    pub fn fill_rect(&mut self, x0: u32, y0: u32, w: u32, h: u32, color: Color) {
        if color.a == 0 {
            return;
        }
        let x1 = (x0 + w).min(self.width);
        let y1 = (y0 + h).min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }

        if color.a == 255 {
            let row_width = self.width as usize;
            for y in y0..y1 {
                let row_start = y as usize * row_width;
                let start = (row_start + x0 as usize) * 4;
                let end = (row_start + x1 as usize) * 4;
                for pixel in self.pixels[start..end].chunks_exact_mut(4) {
                    pixel[0] = color.r;
                    pixel[1] = color.g;
                    pixel[2] = color.b;
                    pixel[3] = color.a;
                }
            }
            return;
        }

        for y in y0..y1 {
            for x in x0..x1 {
                self.blend_pixel(x, y, color);
            }
        }
    }
}
