fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/placard-payload should be two levels below the workspace root");

    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        placard_payload::DICTIONARY_V1,
    );
    let ts = format!("export const DICTIONARY_B64 = \"{b64}\";\n");
    let ts_path = workspace_root.join("sandbox/src/dictionary.ts");
    std::fs::write(&ts_path, &ts).expect("failed to write sandbox/src/dictionary.ts");

    println!(
        "wrote {} bytes to {}",
        placard_payload::DICTIONARY_V1.len(),
        ts_path.display()
    );
}
