mod fetcher;
mod utils;

use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use placard_font::{Font, FontFamily, FontSet, FontStyle, FontWeight};
use placard_render::{CachingFetcher, Diagnostic, ImageFormat, Severity};

const CONNECTOR_CACHE_TTL: Duration = Duration::from_secs(300);

fn make_fetcher() -> CachingFetcher<fetcher::UreqFetcher> {
    CachingFetcher::new(fetcher::UreqFetcher::new(), CONNECTOR_CACHE_TTL)
}

fn print_diagnostics(diagnostics: &[Diagnostic]) {
    for diag in diagnostics {
        let label = match diag.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        eprintln!("{label}: {}", diag.message);
    }
}

struct Args {
    input: PathBuf,
    output: PathBuf,
    width: Option<f32>,
    min_width: Option<f32>,
    max_width: Option<f32>,
    antialiasing: bool,
    font: Option<PathBuf>,
    format: Option<ImageFormat>,
    bench: Option<usize>,
}

const DEFAULT_BENCH_ITERATIONS: usize = 20;

fn print_usage() {
    eprintln!(
        "usage: placard <input.html> [-o <out.webp|.png>] [--width PX] [--min-width PX] [--max-width PX] [--no-anti-aliasing] [--font PATH] [--format webp|png]"
    );
    eprintln!("         (pass - for <input.html> to read from stdin, or - for -o to write raw");
    eprintln!("          image bytes to stdout -- format then defaults to webp; combine as");
    eprintln!("          `placard - -o -` to pipe HTML in and image bytes out)");
    eprintln!(
        "       placard url <input.html> [--base-url URL] [--width PX] [--min-width PX] [--max-width PX] [--no-anti-aliasing] [--format webp|png]"
    );
    eprintln!(
        "         (builds a URL against --base-url; placard doesn't run that service itself)"
    );
    eprintln!(
        "       placard url <input.html> --static [--width PX] [--min-width PX] [--max-width PX] [--no-anti-aliasing] [--font PATH] [--format webp|png]"
    );
    eprintln!("         (--static renders locally and prints a data: URI -- no server needed,");
    eprintln!("          but it won't update if the source HTML changes)");
    eprintln!("       placard presets [--json]");
    eprintln!("         (lists every data-preset connector this build knows how to resolve)");
    eprintln!("       output format defaults to webp; pass -o out.png or --format png for PNG");
    eprintln!("       --width defaults to shrink-wrapping the document's natural content width;");
    eprintln!("       --min-width sets a floor under that (either the auto or explicit width),");
    eprintln!("       --max-width sets a ceiling under the same rules");
    eprintln!(
        "       --no-anti-aliasing disables edge/glyph antialiasing, producing hard-thresholded pixels"
    );
    eprintln!(
        "       --bench [N] renders the input N times (default 20) and prints a per-stage timing"
    );
    eprintln!(
        "          breakdown (html parse, connectors, css parse, style compute, measure width,"
    );
    eprintln!("          layout build, paint, encode) instead of a single-shot render");
}

fn parse_args() -> Result<Args, String> {
    let mut input = None;
    let mut output = None;
    let mut width = None;
    let mut min_width = None;
    let mut max_width = None;
    let mut antialiasing = true;
    let mut font = None;
    let mut format = None;
    let mut bench = None;

    let mut it = env::args().skip(1).peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                output = Some(PathBuf::from(it.next().ok_or("missing value for -o")?))
            }
            "--bench" => {
                let count = match it.peek().and_then(|v| v.parse::<usize>().ok()) {
                    Some(n) => {
                        it.next();
                        n
                    }
                    None => DEFAULT_BENCH_ITERATIONS,
                };
                bench = Some(count);
            }
            "--width" => {
                let v = it.next().ok_or("missing value for --width")?;
                width = Some(v.parse().map_err(|_| format!("invalid width: {v}"))?);
            }
            "--min-width" => {
                let v = it.next().ok_or("missing value for --min-width")?;
                min_width = Some(v.parse().map_err(|_| format!("invalid min-width: {v}"))?);
            }
            "--max-width" => {
                let v = it.next().ok_or("missing value for --max-width")?;
                max_width = Some(v.parse().map_err(|_| format!("invalid max-width: {v}"))?);
            }
            "--no-anti-aliasing" => antialiasing = false,
            "--font" => font = Some(PathBuf::from(it.next().ok_or("missing value for --font")?)),
            "--format" => {
                let v = it.next().ok_or("missing value for --format")?;
                format = Some(
                    ImageFormat::from_extension(&v)
                        .ok_or_else(|| format!("invalid format: {v} (expected png or webp)"))?,
                );
            }
            other if input.is_none() && (other == "-" || !other.starts_with('-')) => {
                input = Some(PathBuf::from(other))
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    let input = input.ok_or("missing input .html path")?;
    let output = output.unwrap_or_else(|| {
        if is_stdio(&input) {
            PathBuf::from("-")
        } else {
            input.with_extension(format.unwrap_or(ImageFormat::DEFAULT).extension())
        }
    });
    Ok(Args {
        input,
        output,
        width,
        min_width,
        max_width,
        antialiasing,
        font,
        format,
        bench,
    })
}

fn fonts_dir_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("PLACARD_FONTS_PATH") {
        return Ok(PathBuf::from(path));
    }

    let exe =
        env::current_exe().map_err(|e| format!("failed to locate current executable: {e}"))?;
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
            "no `fonts/` directory found next to the placard executable (expected {}); \
             place font files there, or pass --font for a single custom font",
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

fn is_stdio(path: &Path) -> bool {
    path == Path::new("-")
}

fn read_html(path: &Path) -> Result<String, String> {
    if is_stdio(path) {
        let mut html = String::new();
        std::io::stdin()
            .read_to_string(&mut html)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        Ok(html)
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))
    }
}

fn fmt_duration(d: Duration) -> String {
    let micros = d.as_nanos() as f64 / 1000.0;
    if micros < 1000.0 {
        format!("{micros:.1}us")
    } else {
        format!("{:.2}ms", micros / 1000.0)
    }
}

fn duration_stats(samples: &[Duration]) -> (Duration, Duration, Duration) {
    let sum: Duration = samples.iter().sum();
    let mean = sum / samples.len() as u32;
    let min = *samples.iter().min().unwrap();
    let max = *samples.iter().max().unwrap();
    (mean, min, max)
}

fn print_bench_row(label: &str, samples: &[Duration]) {
    let (mean, min, max) = duration_stats(samples);
    eprintln!(
        "{label:<16}{:>12}{:>12}{:>12}",
        fmt_duration(mean),
        fmt_duration(min),
        fmt_duration(max)
    );
}

fn run_bench(args: Args, iterations: usize) -> Result<(), String> {
    let font_start = Instant::now();
    let fonts = load_font_set(args.font.as_deref())?;
    let font_load = font_start.elapsed();

    let html = read_html(&args.input)?;
    let fetcher = make_fetcher();
    let format = args
        .format
        .unwrap_or_else(|| placard_render::format_for_path(&args.output));

    let mut stage_samples = Vec::with_capacity(iterations);
    let mut wall_samples = Vec::with_capacity(iterations);
    let mut encode_samples = Vec::with_capacity(iterations);
    let mut last_output = None;

    for _ in 0..iterations {
        let wall_start = Instant::now();
        let output = placard_render::render_to_canvas(
            &html,
            args.width,
            args.min_width,
            args.max_width,
            args.antialiasing,
            &fonts,
            Some(&fetcher),
            None,
        )?;
        wall_samples.push(wall_start.elapsed());

        let encode_start = Instant::now();
        let bytes = format.encode(&output.canvas)?;
        encode_samples.push(encode_start.elapsed());

        stage_samples.push(output.timings);
        last_output = Some((output.canvas, output.diagnostics, bytes));
    }

    let (canvas, diagnostics, bytes) = last_output.expect("iterations is at least 1");
    print_diagnostics(&diagnostics);

    eprintln!("benchmark: {iterations} iterations, {}x{}", canvas.width(), canvas.height());
    eprintln!("{:<16}{:>12}{:>12}{:>12}", "stage", "mean", "min", "max");
    print_bench_row("font load", &[font_load]);

    let stage_count = stage_samples[0].stages().len();
    for i in 0..stage_count {
        let name = stage_samples[0].stages()[i].0;
        let samples: Vec<Duration> = stage_samples.iter().map(|t| t.stages()[i].1).collect();
        print_bench_row(name, &samples);
    }

    let totals: Vec<Duration> = stage_samples.iter().map(|t| t.total()).collect();
    print_bench_row("total", &totals);
    print_bench_row("wall (render)", &wall_samples);
    print_bench_row(&format!("encode ({})", format.extension()), &encode_samples);

    if is_stdio(&args.output) {
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|e| format!("failed to write to stdout: {e}"))?;
    } else {
        std::fs::write(&args.output, &bytes)
            .map_err(|e| format!("failed to write {}: {e}", args.output.display()))?;
        println!("wrote {} ({}x{})", args.output.display(), canvas.width(), canvas.height());
    }
    Ok(())
}

fn run_render(args: Args) -> Result<(), String> {
    if let Some(iterations) = args.bench {
        return run_bench(args, iterations.max(1));
    }

    let html = read_html(&args.input)?;

    let fonts = load_font_set(args.font.as_deref())?;
    let fetcher = make_fetcher();
    let output = placard_render::render_to_canvas(
        &html,
        args.width,
        args.min_width,
        args.max_width,
        args.antialiasing,
        &fonts,
        Some(&fetcher),
        None,
    )?;
    print_diagnostics(&output.diagnostics);
    let canvas = output.canvas;

    let format = args
        .format
        .unwrap_or_else(|| placard_render::format_for_path(&args.output));
    if is_stdio(&args.output) {
        let bytes = format.encode(&canvas)?;
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|e| format!("failed to write to stdout: {e}"))?;
    } else {
        format
            .write(&canvas, &args.output)
            .map_err(|e| format!("failed to write {}: {e}", args.output.display()))?;
        println!(
            "wrote {} ({}x{})",
            args.output.display(),
            canvas.width(),
            canvas.height()
        );
    }
    Ok(())
}

fn run() -> Result<(), String> {
    match env::args().nth(1).as_deref() {
        Some("url") => run_url(),
        Some("presets") => run_presets(),
        _ => run_render(parse_args()?),
    }
}

fn presets_json<'a>(presets: impl Iterator<Item = &'a placard_render::PresetMeta>) -> String {
    let mut out = String::from("[");
    for (i, preset) in presets.enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"preset\":");
        utils::escape(preset.preset, &mut out);
        out.push_str(",\"service\":");
        utils::escape(preset.service, &mut out);
        out.push_str(",\"description\":");
        utils::escape(preset.description, &mut out);
        out.push_str(&format!(",\"numeric\":{}", preset.numeric));
        out.push_str(",\"params\":[");
        for (j, param) in preset.params.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            utils::escape(param.name, &mut out);
            out.push_str(&format!(",\"required\":{}", param.required));
            out.push_str(",\"example\":");
            utils::escape(param.example, &mut out);
            out.push('}');
        }
        out.push_str("]}");
    }
    out.push(']');
    out
}

fn run_presets() -> Result<(), String> {
    let flag = env::args().nth(2);
    let mut presets: Vec<_> = placard_render::all_presets().collect();
    presets.sort_by_key(|p| p.preset);

    if flag.as_deref() == Some("--json") {
        println!("{}", presets_json(presets.into_iter()));
        return Ok(());
    }

    for preset in presets {
        println!("{}  -- {}", preset.preset, preset.description);
        let attrs: Vec<String> = preset
            .params
            .iter()
            .map(|p| {
                let attr = format!(r#"data-{}="{}""#, p.name, p.example);
                if p.required {
                    attr
                } else {
                    format!("[{attr}]")
                }
            })
            .collect();
        println!("    data-preset=\"{}\" {}", preset.preset, attrs.join(" "));
    }
    Ok(())
}

struct UrlArgs {
    input: PathBuf,
    base_url: String,
    width: Option<f32>,
    min_width: Option<f32>,
    max_width: Option<f32>,
    antialiasing: bool,
    font: Option<PathBuf>,
    static_uri: bool,
    format: ImageFormat,
}

fn parse_url_args() -> Result<UrlArgs, String> {
    let mut input = None;
    let mut base_url = "http://localhost:8080".to_string();
    let mut width = None;
    let mut min_width = None;
    let mut max_width = None;
    let mut antialiasing = true;
    let mut font = None;
    let mut static_uri = false;
    let mut format = ImageFormat::DEFAULT;

    let mut it = env::args().skip(2);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--base-url" => base_url = it.next().ok_or("missing value for --base-url")?,
            "--width" => {
                let v = it.next().ok_or("missing value for --width")?;
                width = Some(v.parse().map_err(|_| format!("invalid width: {v}"))?);
            }
            "--min-width" => {
                let v = it.next().ok_or("missing value for --min-width")?;
                min_width = Some(v.parse().map_err(|_| format!("invalid min-width: {v}"))?);
            }
            "--max-width" => {
                let v = it.next().ok_or("missing value for --max-width")?;
                max_width = Some(v.parse().map_err(|_| format!("invalid max-width: {v}"))?);
            }
            "--no-anti-aliasing" => antialiasing = false,
            "--font" => font = Some(PathBuf::from(it.next().ok_or("missing value for --font")?)),
            "--static" => static_uri = true,
            "--format" => {
                let v = it.next().ok_or("missing value for --format")?;
                format = ImageFormat::from_extension(&v)
                    .ok_or_else(|| format!("invalid format: {v} (expected png or webp)"))?;
            }
            other if input.is_none() && !other.starts_with('-') => {
                input = Some(PathBuf::from(other))
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    let input = input.ok_or("missing input .html path")?;
    Ok(UrlArgs {
        input,
        base_url,
        width,
        min_width,
        max_width,
        antialiasing,
        font,
        static_uri,
        format,
    })
}

fn run_url_static(args: &UrlArgs) -> Result<(), String> {
    let html = std::fs::read_to_string(&args.input)
        .map_err(|e| format!("failed to read {}: {e}", args.input.display()))?;

    let fonts = load_font_set(args.font.as_deref())?;
    let fetcher = make_fetcher();

    let output = placard_render::render_to_canvas(
        &html,
        args.width,
        args.min_width,
        args.max_width,
        args.antialiasing,
        &fonts,
        Some(&fetcher),
        None,
    )?;
    print_diagnostics(&output.diagnostics);
    let bytes = args.format.encode(&output.canvas)?;

    let encoded = utils::encode_standard(&bytes);
    println!("data:{};base64,{encoded}", args.format.content_type());
    Ok(())
}

fn run_url() -> Result<(), String> {
    let args = parse_url_args()?;

    if args.static_uri {
        return run_url_static(&args);
    }

    let html = std::fs::read(&args.input)
        .map_err(|e| format!("failed to read {}: {e}", args.input.display()))?;
    let encoded = placard_payload::encode(&html);

    let base = args.base_url.trim_end_matches('/');
    let mut url = format!("{base}/r/{encoded}.{}", args.format.extension());
    let mut params = Vec::new();
    if let Some(width) = args.width {
        params.push(format!("width={width}"));
    }
    if let Some(min_width) = args.min_width {
        params.push(format!("min_width={min_width}"));
    }
    if let Some(max_width) = args.max_width {
        params.push(format!("max_width={max_width}"));
    }
    if !args.antialiasing {
        params.push("antialiasing=0".to_string());
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    println!("{url}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            print_usage();
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use placard_render::PresetMeta;

    fn preset_with_numeric(
        preset: &'static str,
        service: &'static str,
        description: &'static str,
        params: &'static [placard_render::Param],
        numeric: bool,
    ) -> PresetMeta {
        fn resolve(
            _: &std::collections::HashMap<String, String>,
            _: &dyn placard_render::Fetcher,
        ) -> Result<String, String> {
            unreachable!()
        }
        PresetMeta {
            preset,
            service,
            description,
            params,
            numeric,
            resolve,
        }
    }

    #[test]
    fn presets_json_includes_numeric_field() {
        const PARAMS: &[placard_render::Param] = &[];
        let numeric = preset_with_numeric("a", "svc", "d", PARAMS, true);
        let text = preset_with_numeric("b", "svc", "d", PARAMS, false);
        let json = presets_json([&numeric, &text].into_iter());

        assert!(json.contains(r#""preset":"a","service":"svc","description":"d","numeric":true"#));
        assert!(json.contains(r#""preset":"b","service":"svc","description":"d","numeric":false"#));
    }
}
