use placard_raster::Canvas;
use std::path::Path;

/// The image format to encode a render as. WebP is the default -- real
/// compression, versus PNG's intentionally simple, streaming, effectively
/// uncompressed encoder (see `placard-raster`'s `png` module docs) -- kept
/// available as an explicit fallback anywhere a consumer needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Webp,
}

impl ImageFormat {
    pub const DEFAULT: ImageFormat = ImageFormat::Webp;

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(ImageFormat::Png),
            "webp" => Some(ImageFormat::Webp),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Webp => "webp",
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Webp => "image/webp",
        }
    }

    pub fn encode(&self, canvas: &Canvas) -> Result<Vec<u8>, String> {
        match self {
            ImageFormat::Png => Ok(placard_raster::png::encode(canvas)),
            ImageFormat::Webp => placard_raster::webp::encode(canvas),
        }
    }

    pub fn write<P: AsRef<Path>>(&self, canvas: &Canvas, path: P) -> Result<(), String> {
        match self {
            ImageFormat::Png => placard_raster::png::write(canvas, path).map_err(|e| e.to_string()),
            ImageFormat::Webp => placard_raster::webp::write(canvas, path),
        }
    }
}

/// Picks a format from an output path's extension, falling back to
/// [`ImageFormat::DEFAULT`] when the extension is missing or unrecognized.
pub fn format_for_path(path: &Path) -> ImageFormat {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(ImageFormat::from_extension)
        .unwrap_or(ImageFormat::DEFAULT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_extensions_case_insensitively() {
        assert_eq!(ImageFormat::from_extension("png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_extension("PNG"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_extension("webp"), Some(ImageFormat::Webp));
        assert_eq!(ImageFormat::from_extension("WebP"), Some(ImageFormat::Webp));
        assert_eq!(ImageFormat::from_extension("jpg"), None);
    }

    #[test]
    fn format_for_path_defaults_to_webp_when_unrecognized() {
        assert_eq!(format_for_path(Path::new("out.png")), ImageFormat::Png);
        assert_eq!(format_for_path(Path::new("out.webp")), ImageFormat::Webp);
        assert_eq!(format_for_path(Path::new("out")), ImageFormat::Webp);
        assert_eq!(format_for_path(Path::new("out.dat")), ImageFormat::Webp);
    }
}
