//! Windows packaging and portability regressions.
//!
//! These tests lock the installer *configuration* and catch accidental
//! development-machine paths in source and, when present, in built binaries.
//! They do not install Sammy and they do not require a code-signing certificate.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must live under the repository root")
        .to_path_buf()
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&read_text(path))
        .unwrap_or_else(|err| panic!("parse JSON {}: {err}", path.display()))
}

fn workspace_package_version(root: &Path) -> String {
    let cargo = read_text(&root.join("Cargo.toml"));
    let mut in_workspace_package = false;
    for line in cargo.lines() {
        let trimmed = line.trim();
        if trimmed == "[workspace.package]" {
            in_workspace_package = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_workspace_package = false;
            continue;
        }
        if in_workspace_package {
            if let Some(value) = trimmed.strip_prefix("version") {
                let value = value
                    .trim_start_matches(|c: char| c == '=' || c.is_whitespace())
                    .trim_matches('"');
                return value.to_string();
            }
        }
    }
    panic!("workspace.package.version missing from Cargo.toml");
}

fn collect_files(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| {
        panic!("read_dir {}: {err}", dir.display());
    });
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, extensions, out);
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext) {
                out.push(path);
            }
        }
    }
}

fn ascii_contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn ascii_contexts<'a>(haystack: &'a [u8], needle: &[u8]) -> Vec<&'a [u8]> {
    let mut contexts = Vec::new();
    let mut start = 0;
    while let Some(offset) = haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
    {
        let abs = start + offset;
        let from = abs.saturating_sub(80);
        let to = (abs + needle.len() + 160).min(haystack.len());
        contexts.push(&haystack[from..to]);
        start = abs + needle.len();
    }
    contexts
}

#[test]
fn product_versions_match_across_manifests() {
    let root = repo_root();
    let package = read_json(&root.join("package.json"));
    let tauri = read_json(&root.join("src-tauri").join("tauri.conf.json"));
    let npm = package["version"]
        .as_str()
        .expect("package.json version")
        .to_string();
    let tauri_version = tauri["version"]
        .as_str()
        .expect("tauri.conf.json version")
        .to_string();
    let cargo = workspace_package_version(&root);
    assert_eq!(npm, cargo, "package.json and workspace Cargo.toml versions");
    assert_eq!(
        npm, tauri_version,
        "package.json and tauri.conf.json versions"
    );
}

#[test]
fn tauri_bundle_targets_windows_installers_without_signing_secrets() {
    let tauri = read_json(&repo_root().join("src-tauri").join("tauri.conf.json"));
    assert_eq!(tauri["productName"], "Sammy");
    assert_eq!(tauri["identifier"], "app.sammy.desktop");
    let targets = tauri["bundle"]["targets"]
        .as_array()
        .expect("bundle.targets");
    let names: Vec<&str> = targets.iter().filter_map(Value::as_str).collect();
    assert!(names.contains(&"msi"), "MSI target missing: {names:?}");
    assert!(names.contains(&"nsis"), "NSIS target missing: {names:?}");

    let serialized = tauri.to_string();
    for forbidden in [".pfx", ".p12", ".pvk", "PRIVATE KEY"] {
        assert!(
            !serialized.contains(forbidden),
            "tauri.conf.json must not embed signing secrets; found {forbidden}"
        );
    }
}

#[test]
fn gitignore_covers_signing_material() {
    let gitignore = read_text(&repo_root().join(".gitignore"));
    for pattern in ["*.pfx", "*.p12", "*.pem", "*.key", ".signing/"] {
        assert!(
            gitignore.lines().any(|line| line.trim() == pattern),
            ".gitignore must ignore {pattern}"
        );
    }
}

#[test]
fn tauri_bundle_does_not_ship_generated_fixtures() {
    let tauri = read_json(&repo_root().join("src-tauri").join("tauri.conf.json"));
    let serialized = tauri.to_string();
    assert!(
        !serialized.contains("pkc-clickthrough"),
        "tauri.conf.json must not bundle the PKC click-through fixture"
    );
    assert!(
        tauri["bundle"].get("resources").is_none()
            || tauri["bundle"]["resources"]
                .as_array()
                .map(|items| items.is_empty())
                .unwrap_or(false),
        "unexpected extra bundle resources: {:?}",
        tauri["bundle"].get("resources")
    );
    assert!(
        tauri["bundle"].get("externalBin").is_none()
            || tauri["bundle"]["externalBin"]
                .as_array()
                .map(|items| items.is_empty())
                .unwrap_or(false),
        "unexpected extra externalBin entries: {:?}",
        tauri["bundle"].get("externalBin")
    );
}

#[test]
fn source_tree_does_not_hardcode_development_install_paths() {
    let root = repo_root();
    let mut files = Vec::new();
    collect_files(&root.join("src"), &["ts", "tsx", "css"], &mut files);
    collect_files(&root.join("src-tauri").join("src"), &["rs"], &mut files);
    files.push(root.join("src-tauri").join("tauri.conf.json"));

    let mut leaks = Vec::new();
    for path in files {
        let bytes = fs::read(&path).unwrap();
        let scan = if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let text = String::from_utf8_lossy(&bytes);
            text.split("#[cfg(test)]")
                .next()
                .unwrap_or(&text)
                .as_bytes()
                .to_vec()
        } else {
            bytes
        };
        if ascii_contains(&scan, b"ZCodeProject") {
            leaks.push(path.display().to_string());
        }
        if ascii_contains(&scan, b"pkc-clickthrough-fixture") {
            leaks.push(format!("{} (fixture path)", path.display()));
        }
        if ascii_contains(&scan, b"personal-knowledge-corpus-scaffold") {
            leaks.push(format!("{} (PKC absolute path)", path.display()));
        }
        if ascii_contains(&scan, br"D:\dev\StoryKeeper") {
            leaks.push(format!("{} (StoryKeeper absolute path)", path.display()));
        }
    }
    assert!(
        leaks.is_empty(),
        "development-machine paths must not appear in shipped source: {leaks:?}"
    );
}

#[test]
fn release_binaries_if_present_do_not_embed_fixture_or_corpus_paths() {
    let exe = repo_root().join("target").join("release").join("sammy.exe");
    if !exe.exists() {
        return;
    }
    let bytes = fs::read(&exe).unwrap();
    assert!(
        !ascii_contains(&bytes, b"pkc-clickthrough"),
        "sammy.exe must not embed the click-through fixture path"
    );
    assert!(
        !ascii_contains(&bytes, b"personal-knowledge-corpus-scaffold"),
        "sammy.exe must not embed the canonical PKC repository path"
    );
    assert!(
        !ascii_contains(&bytes, br"D:\dev\StoryKeeper"),
        "sammy.exe must not embed the development StoryKeeper bridge path"
    );
}

#[test]
fn release_binary_path_strings_if_present_are_limited_to_vendored_openssl() {
    let exe = repo_root().join("target").join("release").join("sammy.exe");
    if !exe.exists() {
        return;
    }
    let bytes = fs::read(&exe).unwrap();
    let contexts = ascii_contexts(&bytes, b"ZCodeProject");
    for context in &contexts {
        let text = String::from_utf8_lossy(context);
        assert!(
            text.contains("openssl-sys") || text.contains("openssl-build"),
            "unexpected development path in sammy.exe (not vendored OpenSSL metadata): {text}"
        );
    }
}
