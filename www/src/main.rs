mod fetcher;
mod http;

use base64::Engine as _;
use http::{write_response, write_response_with_headers, Request};
use placard_font::{Font, FontFamily, FontSet, FontStyle, FontWeight};
use placard_render::{CachingFetcher, Diagnostic, Fetcher, ImageFormat, MemoryBudget, Severity};
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_REQUEST_SIZE: usize = 96 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REQUEST_TIME: Duration = Duration::from_secs(10);
const CONNECTOR_CACHE_TTL: Duration = Duration::from_secs(300);
const MAX_CONCURRENT_CONNECTIONS: usize = 256;
const MEMORY_BUDGET_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MEMORY_WAIT_TIMEOUT: Duration = Duration::from_secs(4);

const MAX_DECODED_SIZE: usize = 64 * 1024;
const MIN_WIDTH: f32 = 1.0;
const MAX_WIDTH: f32 = 2000.0;

struct Args {
    port: u16,
    font: Option<PathBuf>,
}

fn print_usage() {
    eprintln!("usage: placard-docs-server [--port PORT] [--font PATH]");
    eprintln!("       --port defaults to 8080");
}

fn parse_args() -> Result<Args, String> {
    let mut port = 8080u16;
    let mut font = None;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--port" => {
                let v = it.next().ok_or("missing value for --port")?;
                port = v.parse().map_err(|_| format!("invalid port: {v}"))?;
            }
            "--font" => font = Some(PathBuf::from(it.next().ok_or("missing value for --font")?)),
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    Ok(Args { port, font })
}

fn fonts_dir_path() -> Result<PathBuf, String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("failed to locate current executable: {e}"))?;
    Ok(exe
        .parent()
        .ok_or("current executable has no parent directory")?
        .join("fonts"))
}

fn read_font(dir: &Path, rel: &str) -> Result<Vec<u8>, String> {
    let path = dir.join(rel);
    std::fs::read(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))
}

fn load_font_set(explicit: Option<&Path>) -> Result<FontSet, String> {
    if let Some(path) = explicit {
        let data = std::fs::read(path)
            .map_err(|e| format!("failed to read font {}: {e}", path.display()))?;
        let font = Font::parse(&data)
            .map_err(|e| format!("failed to parse font {}: {e}", path.display()))?;
        return Ok(FontSet::new(font));
    }

    let dir = fonts_dir_path()?;
    if !dir.is_dir() {
        return Err(format!(
            "no `fonts/` directory found next to the placard-docs-server executable \
             (expected {}); place font files there, or pass --font for a single custom font",
            dir.display()
        ));
    }

    let base_data = read_font(&dir, "inter/Inter-Regular.ttf")?;
    let base_font = Font::parse(&base_data)
        .map_err(|e| format!("failed to parse inter/Inter-Regular.ttf: {e}"))?;
    let mut fonts = FontSet::new(base_font);

    scan_named_fonts(&dir, &mut fonts);
    Ok(fonts)
}

fn scan_named_fonts(dir: &Path, fonts: &mut FontSet) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let Ok(files) = std::fs::read_dir(&path) else {
            continue;
        };
        for file in files.flatten() {
            let file_path = file.path();
            let is_font = file_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"))
                .unwrap_or(false);
            if !is_font {
                continue;
            }

            let stem = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let weight = if stem.contains("bold") {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            };
            let style = if stem.contains("italic") || stem.contains("oblique") {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            };

            let Ok(data) = std::fs::read(&file_path) else {
                eprintln!("warning: failed to read {}", file_path.display());
                continue;
            };
            match Font::parse(&data) {
                Ok(font) => {
                    let family_name = font
                        .family_name()
                        .map(str::to_string)
                        .unwrap_or_else(|| dir_name.to_string());
                    fonts.insert(FontFamily::Named(family_name), weight, style, font);
                }
                Err(e) => eprintln!("warning: failed to parse {}: {e}", file_path.display()),
            }
        }
    }
}

fn make_fetcher() -> CachingFetcher<fetcher::UreqFetcher> {
    CachingFetcher::new(fetcher::UreqFetcher::new(), CONNECTOR_CACHE_TTL)
}

fn static_asset(path: &str) -> Option<(&'static str, &'static [u8])> {
    match path {
        "/" => Some((
            "text/html; charset=utf-8",
            include_bytes!("../routes/index.html"),
        )),
        "/style.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../routes/style.css"),
        )),
        "/favicon.svg" => Some(("image/svg+xml", include_bytes!("../routes/favicon.svg"))),
        "/assets/theme.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../routes/assets/theme.css"),
        )),
        "/assets/theme.js" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../routes/assets/theme.js"),
        )),
        "/assets/geist-sans.woff2" => Some((
            "font/woff2",
            include_bytes!("../routes/assets/geist-sans.woff2"),
        )),
        "/assets/geist-mono.woff2" => Some((
            "font/woff2",
            include_bytes!("../routes/assets/geist-mono.woff2"),
        )),
        "/connectors" => Some((
            "text/html; charset=utf-8",
            include_bytes!("../routes/connectors/index.html"),
        )),
        "/connectors/style.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../routes/connectors/style.css"),
        )),
        "/connectors/script.js" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../routes/connectors/script.js"),
        )),
        "/sandbox" => Some((
            "text/html; charset=utf-8",
            include_bytes!("../routes/sandbox/index.html"),
        )),
        "/sandbox/style.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../routes/sandbox/style.css"),
        )),
        "/sandbox/script.js" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../routes/sandbox/script.js"),
        )),
        "/sandbox/editor.js" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../routes/sandbox/editor.js"),
        )),
        _ => None,
    }
}

fn serve_static(stream: &mut TcpStream, url_path: &str) {
    let path = url_path
        .strip_suffix('/')
        .filter(|p| !p.is_empty())
        .unwrap_or(url_path);

    match static_asset(path) {
        Some((content_type, bytes)) => {
            let _ = write_response(stream, 200, "OK", content_type, bytes);
        }
        None => {
            let _ = write_response(stream, 404, "Not Found", "text/plain", b"not found");
        }
    }
}

fn presets_json() -> Vec<u8> {
    let mut presets: Vec<_> = placard_render::all_presets().collect();
    presets.sort_by_key(|p| p.preset);

    let value = serde_json::Value::Array(
        presets
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "preset": p.preset,
                    "service": p.service,
                    "description": p.description,
                    "numeric": p.numeric,
                    "params": p.params.iter().map(|param| serde_json::json!({
                        "name": param.name,
                        "required": param.required,
                        "example": param.example,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    );
    serde_json::to_vec(&value).unwrap_or_default()
}

fn parse_pixel_param(request: &Request, name: &str) -> Result<Option<f32>, String> {
    match request.query_param(name) {
        Some(raw) => match raw.parse::<f32>() {
            Ok(v) if (MIN_WIDTH..=MAX_WIDTH).contains(&v) => Ok(Some(v)),
            _ => Err(format!(
                "{name} must be between {MIN_WIDTH} and {MAX_WIDTH}"
            )),
        },
        None => Ok(None),
    }
}

fn render_route(
    stream: &mut TcpStream,
    payload: &str,
    request: &Request,
    fonts: &FontSet,
    fetcher: &dyn Fetcher,
    budget: &Arc<MemoryBudget>,
) {
    let (format, payload) = if let Some(p) = payload.strip_suffix(".webp") {
        (ImageFormat::Webp, p)
    } else if let Some(p) = payload.strip_suffix(".png") {
        (ImageFormat::Png, p)
    } else {
        (ImageFormat::DEFAULT, payload)
    };

    let width = match parse_pixel_param(request, "width") {
        Ok(w) => w,
        Err(msg) => {
            let _ = write_response(stream, 400, "Bad Request", "text/plain", msg.as_bytes());
            return;
        }
    };
    let min_width = match parse_pixel_param(request, "min_width") {
        Ok(w) => w,
        Err(msg) => {
            let _ = write_response(stream, 400, "Bad Request", "text/plain", msg.as_bytes());
            return;
        }
    };

    let decoded = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload) {
        Ok(d) => d,
        Err(_) => {
            let _ = write_response(
                stream,
                400,
                "Bad Request",
                "text/plain",
                b"invalid base64url payload",
            );
            return;
        }
    };

    if decoded.len() > MAX_DECODED_SIZE {
        let _ = write_response(
            stream,
            413,
            "Payload Too Large",
            "text/plain",
            b"decoded payload too large",
        );
        return;
    }

    let html = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => {
            let _ = write_response(
                stream,
                400,
                "Bad Request",
                "text/plain",
                b"payload is not valid UTF-8",
            );
            return;
        }
    };

    let output = match placard_render::render_to_canvas(
        &html,
        width,
        min_width,
        fonts,
        Some(fetcher),
        Some(budget),
    ) {
        Ok(o) => o,
        Err(e) => {
            let (status, reason) = if e == placard_render::AT_CAPACITY_ERROR {
                (503, "Service Unavailable")
            } else {
                (422, "Unprocessable Entity")
            };
            let _ = write_response(stream, status, reason, "text/plain", e.as_bytes());
            return;
        }
    };
    let canvas = output.canvas;

    if canvas.width() as f32 > MAX_WIDTH {
        let _ = write_response(
            stream,
            422,
            "Unprocessable Entity",
            "text/plain",
            b"auto-resolved width exceeds the 2000px cap; pass an explicit width or shorten the content",
        );
        return;
    }

    let bytes = match format.encode(&canvas) {
        Ok(b) => b,
        Err(e) => {
            let _ = write_response(
                stream,
                422,
                "Unprocessable Entity",
                "text/plain",
                e.as_bytes(),
            );
            return;
        }
    };

    let diagnostics_header = diagnostics_json(&output.diagnostics);
    let _ = write_response_with_headers(
        stream,
        200,
        "OK",
        format.content_type(),
        &bytes,
        &[("X-Placard-Diagnostics", &diagnostics_header)],
    );
}

fn diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    let value = serde_json::Value::Array(
        diagnostics
            .iter()
            .map(|d| {
                let severity = match d.severity {
                    Severity::Warning => "warning",
                    Severity::Error => "error",
                };
                serde_json::json!({ "severity": severity, "message": d.message })
            })
            .collect(),
    );
    serde_json::to_string(&value).unwrap_or_else(|_| "[]".to_string())
}

fn handle(
    stream: &mut TcpStream,
    request: &Request,
    fonts: &FontSet,
    fetcher: &dyn Fetcher,
    budget: &Arc<MemoryBudget>,
) {
    if request.method != "GET" {
        let _ = write_response(
            stream,
            405,
            "Method Not Allowed",
            "text/plain",
            b"only GET is supported",
        );
        return;
    }

    if request.path == "/presets" {
        let body = presets_json();
        let _ = write_response(stream, 200, "OK", "application/json", &body);
        return;
    }

    if let Some(payload) = request.path.strip_prefix("/r/") {
        render_route(stream, payload, request, fonts, fetcher, budget);
        return;
    }

    serve_static(stream, &request.path);
}

fn handle_connection(
    mut stream: TcpStream,
    fonts: &FontSet,
    fetcher: &dyn Fetcher,
    budget: &Arc<MemoryBudget>,
) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));

    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(reader_stream);
    let deadline = Instant::now() + MAX_REQUEST_TIME;

    match http::read_request(&mut reader, MAX_REQUEST_SIZE, deadline) {
        Ok(request) => handle(&mut stream, &request, fonts, fetcher, budget),
        Err(http::RequestError::TooLarge) => {
            let _ = write_response(
                &mut stream,
                413,
                "Payload Too Large",
                "text/plain",
                b"request too large",
            );
        }
        Err(http::RequestError::TimedOut) => {
            let _ = write_response(
                &mut stream,
                408,
                "Request Timeout",
                "text/plain",
                b"request took too long to send",
            );
        }
        Err(_) => {
            let _ = write_response(
                &mut stream,
                400,
                "Bad Request",
                "text/plain",
                b"malformed request",
            );
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let fonts = Arc::new(load_font_set(args.font.as_deref())?);
    let fetcher: Arc<dyn Fetcher> = Arc::new(make_fetcher());
    let budget = Arc::new(MemoryBudget::new(MEMORY_BUDGET_BYTES, MEMORY_WAIT_TIMEOUT));

    let listener =
        TcpListener::bind(("0.0.0.0", args.port)).map_err(|e| format!("bind failed: {e}"))?;
    println!(
        "placard-docs-server listening on http://0.0.0.0:{}",
        args.port
    );

    let active_connections = Arc::new(AtomicUsize::new(0));

    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else { continue };

        if active_connections.fetch_add(1, Ordering::SeqCst) >= MAX_CONCURRENT_CONNECTIONS {
            active_connections.fetch_sub(1, Ordering::SeqCst);
            let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
            let _ = write_response(
                &mut stream,
                503,
                "Service Unavailable",
                "text/plain",
                b"server is at capacity, try again shortly",
            );
            continue;
        }

        let fonts = Arc::clone(&fonts);
        let fetcher = Arc::clone(&fetcher);
        let budget = Arc::clone(&budget);
        let active_connections = Arc::clone(&active_connections);
        std::thread::spawn(move || {
            handle_connection(stream, &fonts, fetcher.as_ref(), &budget);
            active_connections.fetch_sub(1, Ordering::SeqCst);
        });
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            print_usage();
            std::process::ExitCode::FAILURE
        }
    }
}
