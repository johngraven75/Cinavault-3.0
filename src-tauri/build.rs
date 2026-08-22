use std::{env, fs, path::PathBuf};

fn main() {
    write_legacy_metadata_without_command_attrs();
    write_metadata_ext_without_repaired_command_attrs();
    write_metadata_guard_without_command_attrs();
    write_enrichment_with_atomic_nfo();
    tauri_build::build()
}

fn manifest_and_output_paths(source_name: &str, output_name: &str) -> (PathBuf, PathBuf) {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo"),
    );
    let source_path = manifest_dir.join("src").join(source_name);
    let out_path =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo")).join(output_name);
    (source_path, out_path)
}

fn normalized_source(path: &PathBuf) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

fn strip_command_attribute(source: String, function_name: &str) -> (String, usize) {
    let mut sanitized = source;
    let mut stripped = 0usize;
    for prefix in ["pub fn", "pub async fn"] {
        let needle = format!("#[tauri::command]\n{prefix} {function_name}");
        if sanitized.contains(&needle) {
            sanitized = sanitized.replace(&needle, &format!("{prefix} {function_name}"));
            stripped += 1;
        }
    }
    (sanitized, stripped)
}

fn strip_expected_commands(mut source: String, command_functions: &[&str], label: &str) -> String {
    let mut stripped = 0usize;
    for function_name in command_functions {
        let result = strip_command_attribute(source, function_name);
        source = result.0;
        stripped += result.1;
    }
    assert_eq!(
        stripped,
        command_functions.len(),
        "{label} command sanitization did not find every expected command"
    );
    source
}

fn write_legacy_metadata_without_command_attrs() {
    println!("cargo:rerun-if-changed=src/metadata.rs");
    let (source_path, out_path) =
        manifest_and_output_paths("metadata.rs", "metadata_without_commands.rs");
    let source = normalized_source(&source_path);
    let sanitized = strip_expected_commands(
        source,
        &[
            "get_metadata_providers",
            "fetch_metadata",
            "search_metadata",
            "check_media_item_metadata",
            "get_provider_status",
            "test_api_key",
            "set_api_key",
            "get_api_keys",
        ],
        "legacy metadata",
    );
    fs::write(out_path, sanitized).expect("failed to write sanitized legacy metadata module");
}

fn write_metadata_ext_without_repaired_command_attrs() {
    println!("cargo:rerun-if-changed=src/metadata_ext.rs");
    let (source_path, out_path) = manifest_and_output_paths(
        "metadata_ext.rs",
        "metadata_ext_without_repaired_commands.rs",
    );
    let source = normalized_source(&source_path);
    let sanitized = strip_expected_commands(
        source,
        &["check_media_item_metadata"],
        "metadata extension repaired wrapper",
    );
    fs::write(out_path, sanitized).expect("failed to write sanitized metadata extension module");
}

fn write_metadata_guard_without_command_attrs() {
    println!("cargo:rerun-if-changed=src/metadata_guard.rs");
    let (source_path, out_path) =
        manifest_and_output_paths("metadata_guard.rs", "metadata_guard_without_commands.rs");
    let source = normalized_source(&source_path);
    let sanitized = strip_expected_commands(
        source,
        &[
            "get_metadata_providers",
            "fetch_metadata",
            "search_metadata",
            "check_media_item_metadata",
            "get_provider_status",
            "test_api_key",
            "set_api_key",
            "get_api_keys",
        ],
        "metadata guard",
    );
    fs::write(out_path, sanitized).expect("failed to write sanitized metadata guard module");
}

fn write_enrichment_with_atomic_nfo() {
    println!("cargo:rerun-if-changed=src/enrichment.rs");
    println!("cargo:rerun-if-changed=src/atomic_file.rs");
    let (source_path, out_path) =
        manifest_and_output_paths("enrichment.rs", "enrichment_atomic.rs");
    let source = normalized_source(&source_path);
    let direct_write = "    std::fs::write(&nfo_path, nfo_content.as_bytes())\n        .map_err(|e| format!(\"nfo write failed: {e}\"))?;";
    let atomic_write = "    crate::atomic_file::write_verified_atomic(&nfo_path, nfo_content.as_bytes())\n        .map_err(|e| format!(\"nfo write failed: {e}\"))?;";
    let occurrences = source.matches(direct_write).count();
    assert_eq!(
        occurrences, 1,
        "enrichment atomic NFO transformation expected exactly one direct NFO write"
    );
    let transformed = source.replacen(direct_write, atomic_write, 1);
    let sanitized = strip_expected_commands(
        transformed,
        &["run_library_enrichment"],
        "legacy enrichment",
    );
    fs::write(out_path, sanitized).expect("failed to write atomic enrichment module");
}
