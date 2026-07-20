mod format;

use placard_font::FontSet;
use placard_html::{Dom, NodeData, NodeId};
use placard_raster::{Canvas, Color};
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

pub use format::{ImageFormat, format_for_path};
pub use placard_connectors::{CachingFetcher, Fetcher, Param, PresetMeta, all_presets};
pub use placard_style::{Diagnostic, Severity};

const MAX_DIAGNOSTICS: usize = 200;

pub struct RenderOutput {
    pub canvas: Canvas,
    pub diagnostics: Vec<Diagnostic>,
    _reservation: Option<Reservation>,
}

pub struct MemoryBudget {
    limit: u64,
    max_wait: Duration,
    used: Mutex<u64>,
    freed: Condvar,
}

pub const AT_CAPACITY_ERROR: &str = "server is at capacity: too many large renders in flight and none finished within the wait budget; try again shortly or request a smaller width";

impl MemoryBudget {
    pub fn new(limit_bytes: u64, max_wait: Duration) -> Self {
        Self {
            limit: limit_bytes,
            max_wait,
            used: Mutex::new(0),
            freed: Condvar::new(),
        }
    }

    fn try_reserve(self: &Arc<Self>, bytes: u64) -> Option<Reservation> {
        if bytes > self.limit {
            return None;
        }
        let deadline = Instant::now() + self.max_wait;
        let mut used = self.used.lock().unwrap();
        loop {
            if *used + bytes <= self.limit {
                *used += bytes;
                return Some(Reservation {
                    budget: Arc::clone(self),
                    bytes,
                });
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            used = self.freed.wait_timeout(used, remaining).unwrap().0;
        }
    }
}

struct Reservation {
    budget: Arc<MemoryBudget>,
    bytes: u64,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        {
            let mut used = self.budget.used.lock().unwrap();
            *used -= self.bytes;
        }
        self.budget.freed.notify_all();
    }
}

fn to_font_db_family(family: &placard_style::FontFamily) -> placard_font::FontFamily {
    match family {
        placard_style::FontFamily::SansSerif => placard_font::FontFamily::SansSerif,
        placard_style::FontFamily::Serif => placard_font::FontFamily::Serif,
        placard_style::FontFamily::Monospace => placard_font::FontFamily::Monospace,
        placard_style::FontFamily::Named(name) => placard_font::FontFamily::Named(name.clone()),
    }
}

fn font_diagnostics(styles: &[placard_style::ComputedStyle], fonts: &FontSet) -> Vec<Diagnostic> {
    let mut missing: BTreeMap<String, &str> = BTreeMap::new();
    for style in styles {
        for family in &style.font_family {
            if let placard_style::FontFamily::Named(name) = family
                && !fonts.has_family(&to_font_db_family(family))
            {
                missing
                    .entry(name.to_ascii_lowercase())
                    .or_insert(name.as_str());
            }
        }
    }

    if missing.is_empty() {
        return Vec::new();
    }
    let available = fonts.available_families().join(", ");
    missing
        .into_values()
        .map(|name| {
            Diagnostic::warning(format!(
                "unrecognized font family \"{name}\" -- falling back to the default; \
                 available fonts: {available}"
            ))
        })
        .collect()
}

pub fn extract_styles(dom: &Dom) -> String {
    let mut css = String::new();
    collect_styles(dom, dom.root(), &mut css);
    css
}

fn collect_styles(dom: &Dom, node: NodeId, css: &mut String) {
    if let NodeData::Element { tag, .. } = dom.data(node) {
        if tag == "style" {
            if let Some(text_child) = dom.first_child(node) {
                if let Some(text) = dom.text(text_child) {
                    css.push_str(text);
                    css.push('\n');
                }
            }
        }
    }
    for child in dom.children(node) {
        collect_styles(dom, child, css);
    }
}

pub const MAX_CANVAS_PIXELS: u64 = 3840 * 2160;

fn clamp_width(width: f32, min_width: Option<f32>, max_width: Option<f32>) -> f32 {
    let width = width.max(min_width.unwrap_or(0.0));
    match max_width {
        Some(max) => width.min(max),
        None => width,
    }
}

pub fn render_to_canvas(
    html: &str,
    width: Option<f32>,
    min_width: Option<f32>,
    max_width: Option<f32>,
    fonts: &FontSet,
    fetcher: Option<&dyn Fetcher>,
    budget: Option<&Arc<MemoryBudget>>,
) -> Result<RenderOutput, String> {
    if let (Some(min), Some(max)) = (min_width, max_width)
        && min > max
    {
        return Err(format!(
            "--min-width ({min}) cannot exceed --max-width ({max})"
        ));
    }

    let mut dom = placard_html::parse(html);
    if let Some(fetcher) = fetcher {
        placard_connectors::resolve(&mut dom, fetcher);
    }
    let css_text = extract_styles(&dom);
    let (stylesheet, mut diagnostics) = placard_css::parse_with_diagnostics(&css_text);
    let styles = placard_style::compute(&dom, &stylesheet);

    diagnostics.extend(placard_style::lint(&dom, &stylesheet));
    diagnostics.extend(font_diagnostics(&styles, fonts));
    diagnostics.truncate(MAX_DIAGNOSTICS);

    let auto_width = width.is_none();

    let layout_width = match width {
        Some(w) => w,
        None => placard_layout::measure_document_width(&dom, &styles, fonts).ceil(),
    };
    let layout_width = clamp_width(layout_width, min_width, max_width).max(1.0);

    let tree = placard_layout::build(&dom, &styles, fonts, layout_width);

    let height = tree.max_extent_y().round().max(1.0) as u32;
    let canvas_width = if auto_width {
        clamp_width(tree.max_extent_x().round(), min_width, max_width).max(1.0) as u32
    } else {
        layout_width as u32
    };

    let pixels = canvas_width as u64 * height as u64;
    if pixels > MAX_CANVAS_PIXELS {
        return Err(format!(
            "rendered canvas would be {canvas_width}x{height} ({pixels} px), which exceeds the {MAX_CANVAS_PIXELS}px cap; \
             pass a smaller --width or shorten the document"
        ));
    }

    let reservation = match budget {
        Some(budget) => Some(
            budget
                .try_reserve(pixels * 4)
                .ok_or_else(|| AT_CAPACITY_ERROR.to_string())?,
        ),
        None => None,
    };

    let mut canvas = Canvas::new(canvas_width, height);
    canvas.fill(Color::rgba(255, 255, 255, 255));
    placard_paint::paint(&mut canvas, &tree, fonts);
    Ok(RenderOutput {
        canvas,
        diagnostics,
        _reservation: reservation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use placard_font::{Font, FontSet};

    fn test_fonts() -> FontSet {
        let data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
            .expect("failed to read font");
        FontSet::new(Font::parse(&data).expect("failed to parse font"))
    }

    #[test]
    fn memory_budget_rejects_a_reservation_larger_than_the_whole_limit() {
        let budget = Arc::new(MemoryBudget::new(1000, Duration::from_millis(50)));
        assert!(budget.try_reserve(1001).is_none());
    }

    #[test]
    fn memory_budget_grants_reservations_that_fit() {
        let budget = Arc::new(MemoryBudget::new(1000, Duration::from_millis(50)));
        assert!(budget.try_reserve(400).is_some());
        assert!(budget.try_reserve(600).is_some());
    }

    #[test]
    fn memory_budget_times_out_when_space_never_frees() {
        let budget = Arc::new(MemoryBudget::new(1000, Duration::from_millis(100)));
        let _held = budget.try_reserve(1000).expect("should fit exactly");

        let start = Instant::now();
        let result = budget.try_reserve(1);
        let elapsed = start.elapsed();

        assert!(result.is_none());
        assert!(elapsed >= Duration::from_millis(100));
    }

    #[test]
    fn memory_budget_wakes_up_once_space_frees_before_the_deadline() {
        let budget = Arc::new(MemoryBudget::new(1000, Duration::from_secs(5)));
        let held = budget.try_reserve(1000).expect("should fit exactly");

        let waiter_budget = Arc::clone(&budget);
        let waiter = std::thread::spawn(move || {
            let start = Instant::now();
            let reservation = waiter_budget.try_reserve(1000);
            (reservation.is_some(), start.elapsed())
        });

        std::thread::sleep(Duration::from_millis(150));
        drop(held);

        let (got_reservation, elapsed) = waiter.join().unwrap();
        assert!(
            got_reservation,
            "waiter should have been granted a reservation once space freed"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "waiter should have woken up well before its deadline, took {elapsed:?}"
        );
    }

    #[test]
    fn render_to_canvas_reports_the_at_capacity_error_when_budget_is_exhausted() {
        let fonts = test_fonts();
        let budget = Arc::new(MemoryBudget::new(1, Duration::from_millis(50)));
        let html = "<body style=\"margin:0\"><div class=\"a\"></div></body>\
                     <style>div.a { width: 10px; height: 10px; }</style>";
        match render_to_canvas(html, Some(400.0), None, None, &fonts, None, Some(&budget)) {
            Err(err) => assert_eq!(err, AT_CAPACITY_ERROR),
            Ok(_) => panic!("expected the render to be rejected for exceeding the budget"),
        }
    }

    #[test]
    fn ordinary_document_renders_successfully() {
        let fonts = test_fonts();
        let canvas = render_to_canvas("<div>hello</div>", Some(400.0), None, None, &fonts, None, None)
            .expect("should render")
            .canvas;
        assert_eq!(canvas.width(), 400);
    }

    #[test]
    fn oversized_canvas_is_rejected_with_an_error() {
        let fonts = test_fonts();
        let html = "<div class=\"tall\"></div><style>div.tall { height: 9000000px; }</style>";
        match render_to_canvas(html, Some(400.0), None, None, &fonts, None, None) {
            Err(err) => assert!(err.contains("exceeds"), "unexpected error message: {err}"),
            Ok(_) => panic!("expected oversized canvas to be rejected"),
        }
    }

    #[test]
    fn omitted_width_shrinks_to_fit_content() {
        let fonts = test_fonts();
        let canvas = render_to_canvas(
            "<body style=\"margin: 0\"><div class=\"a\"></div></body><style>div.a { width: 120px; height: 10px; }</style>",
            None,
            None,
            None,
            &fonts,
            None,
            None,
        )
        .expect("should render")
        .canvas;
        assert_eq!(canvas.width(), 120);
    }

    #[test]
    fn min_width_floors_the_auto_resolved_width() {
        let fonts = test_fonts();
        let canvas = render_to_canvas(
            "<body style=\"margin: 0\"><div class=\"a\"></div></body><style>div.a { width: 20px; height: 10px; }</style>",
            None,
            Some(200.0),
            None,
            &fonts,
            None,
            None,
        )
        .expect("should render")
        .canvas;
        assert_eq!(canvas.width(), 200);
    }

    #[test]
    fn max_width_ceils_the_auto_resolved_width() {
        let fonts = test_fonts();
        let canvas = render_to_canvas(
            "<body style=\"margin: 0\"><div class=\"a\"></div></body><style>div.a { width: 500px; height: 10px; }</style>",
            None,
            None,
            Some(200.0),
            &fonts,
            None,
            None,
        )
        .expect("should render")
        .canvas;
        assert_eq!(canvas.width(), 200);
    }

    #[test]
    fn max_width_ceils_an_explicit_width() {
        let fonts = test_fonts();
        let canvas = render_to_canvas(
            "<div>hello</div>",
            Some(500.0),
            None,
            Some(200.0),
            &fonts,
            None,
            None,
        )
        .expect("should render")
        .canvas;
        assert_eq!(canvas.width(), 200);
    }

    #[test]
    fn min_width_greater_than_max_width_is_rejected() {
        let fonts = test_fonts();
        match render_to_canvas(
            "<div>hello</div>",
            None,
            Some(300.0),
            Some(200.0),
            &fonts,
            None,
            None,
        ) {
            Err(err) => assert!(
                err.contains("min-width") && err.contains("max-width"),
                "unexpected error message: {err}"
            ),
            Ok(_) => panic!("expected min-width > max-width to be rejected"),
        }
    }

    #[test]
    fn auto_width_flex_row_has_no_residual_gap_past_a_ceiled_safety_margin() {
        let fonts = test_fonts();
        let canvas = render_to_canvas(
            "<body style=\"margin: 0\"><div class=\"row\"><div class=\"a\"></div><div class=\"b\"></div></div></body>\
             <style>div.row { display: flex; } \
             div.a { width: 50px; height: 10px; background: green; } \
             div.b { width: 49.1px; height: 10px; background: green; }</style>",
            None,
            None,
            None,
            &fonts,
            None,
            None,
        )
        .expect("should render")
        .canvas;

        // Natural content is 99.1px wide -- ceiling that (the safety
        // margin `build` needs to avoid wrapping) would give 100, but
        // nothing is painted in that extra column, since the transparent
        // flex row fills to whatever width it's built against regardless
        // of whether its items actually reach that far.
        assert_eq!(canvas.width(), 99);
        assert_eq!(
            canvas.get_pixel(98, 5),
            placard_raster::Color::rgba(0, 128, 0, 255)
        );
    }

    #[test]
    fn fractional_content_height_rounds_the_canvas_to_meet_it() {
        let fonts = test_fonts();
        let canvas = render_to_canvas(
            "<body style=\"margin: 0\"><div class=\"a\"></div></body><style>div.a { width: 50px; height: 39.6px; background: green; }</style>",
            None,
            None,
            None,
            &fonts,
            None,
            None,
        )
        .expect("should render")
        .canvas;

        // Rounds up to 40, and the box's own bottom edge -- painted with
        // the same rounding -- must reach every one of those rows, not
        // leave the last one an untouched sliver of canvas background.
        assert_eq!(canvas.height(), 40);
        assert_eq!(
            canvas.get_pixel(10, 39),
            placard_raster::Color::rgba(0, 128, 0, 255)
        );
    }

    #[test]
    fn ordinary_document_has_no_diagnostics() {
        let fonts = test_fonts();
        let output = render_to_canvas("<div>hello</div>", Some(400.0), None, None, &fonts, None, None)
            .expect("should render");
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn unrecognized_css_property_surfaces_a_diagnostic() {
        let fonts = test_fonts();
        let html = "<div class=\"a\">hi</div><style>div.a { cursor: pointer; }</style>";
        let output =
            render_to_canvas(html, Some(400.0), None, None, &fonts, None, None).expect("should render");
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.message.contains("cursor")),
            "expected a diagnostic mentioning `cursor`, got {:?}",
            output.diagnostics
        );
    }

    #[test]
    fn unrecognized_font_family_lists_available_fonts() {
        let fonts = test_fonts();
        let html = "<div style=\"font-family: 'some nonexistent font'\">hi</div>";
        let output =
            render_to_canvas(html, Some(400.0), None, None, &fonts, None, None).expect("should render");
        let diag = output
            .diagnostics
            .iter()
            .find(|d| d.message.contains("some nonexistent font"))
            .expect("expected a diagnostic about the missing font family");
        assert!(diag.message.contains("sans-serif"));
    }

    #[test]
    fn unrecognized_font_family_diagnostic_preserves_authored_case() {
        let fonts = test_fonts();
        let html = "<div style=\"font-family: 'Some Nonexistent Font'\">hi</div>";
        let output =
            render_to_canvas(html, Some(400.0), None, None, &fonts, None, None).expect("should render");
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.message.contains("Some Nonexistent Font")),
            "expected the diagnostic to keep the author's casing, got {:?}",
            output.diagnostics
        );
    }

    #[test]
    fn unrecognized_font_family_dedupes_case_insensitively() {
        let fonts = test_fonts();
        let html = "<div style=\"font-family: MadeUpFont\">a</div>\
                     <div style=\"font-family: MADEUPFONT\">b</div>\
                     <div style=\"font-family: madeupfont\">c</div>";
        let output =
            render_to_canvas(html, Some(400.0), None, None, &fonts, None, None).expect("should render");
        let matching: Vec<_> = output
            .diagnostics
            .iter()
            .filter(|d| d.message.to_ascii_lowercase().contains("madeupfont"))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "expected one deduplicated diagnostic, got {matching:?}"
        );
    }
}
