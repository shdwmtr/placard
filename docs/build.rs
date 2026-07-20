use std::fs;
use std::path::{Path, PathBuf};

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vendor_root = manifest_dir.join("routes/sandbox/vendor/monaco");
    println!("cargo:rerun-if-changed={}", vendor_root.display());

    let mut files = Vec::new();
    collect_files(&vendor_root, &mut files);

    let mut generated = String::new();
    generated.push_str("pub static MONACO_ASSETS: &[(&str, &str, &[u8])] = &[\n");
    for path in &files {
        let rel = path
            .strip_prefix(&vendor_root)
            .expect("file must live under vendor_root")
            .to_str()
            .expect("vendored monaco paths must be valid UTF-8")
            .replace('\\', "/");
        let url_path = format!("/sandbox/vendor/monaco/{rel}");
        let ctype = content_type(path);
        let abs_path = path
            .to_str()
            .expect("vendored monaco paths must be valid UTF-8");
        generated.push_str(&format!(
            "    ({url_path:?}, {ctype:?}, include_bytes!({abs_path:?})),\n"
        ));
    }
    generated.push_str("];\n");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo"));
    fs::write(out_dir.join("monaco_assets.rs"), generated)
        .expect("failed to write generated monaco asset table");
}
