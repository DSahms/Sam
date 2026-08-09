//! `sammy-cli` — command-line utility.
//!
//! Per directive §21, Sammy ships a command-line validator for corpus packages.
//! The full validator lands in Phase 6; for now this binary exists so the
//! build target is in place and `sammy-cli --help` works for smoke checks.

use std::process::ExitCode;

const HELP: &str = "sammy-cli — Sammy command-line utility

Usage:
  sammy-cli corpus validate <package>
  sammy-cli --version
  sammy-cli --help

Subcommands:
  corpus validate   Validate a Sammy corpus export package against the
                    documented schema (Phase 6).
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--version") => {
            println!("sammy-cli {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | None => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Some("corpus") => corpus_main(&args[2..]),
        Some(other) => {
            eprintln!("sammy-cli: unknown subcommand '{other}'");
            eprintln!("\n{HELP}");
            ExitCode::from(2)
        }
    }
}

/// `sammy-cli corpus validate <package>` — validates a Sammy corpus export
/// package against the documented schema (directive §21). Returns exit 0 on
/// success, 1 on validation failure, 2 on usage error.
fn corpus_main(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("validate") => {
            let path = match args.get(1) {
                Some(p) => p,
                None => {
                    eprintln!("usage: sammy-cli corpus validate <package.json>");
                    return ExitCode::from(2);
                }
            };
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("sammy-cli: cannot read {path}: {e}");
                    return ExitCode::from(2);
                }
            };
            match sammy_lib::corpus::validate_package_bytes(&bytes) {
                Ok(()) => {
                    println!("valid: {path}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("invalid: {path}: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some(other) => {
            eprintln!("sammy-cli corpus: unknown subcommand '{other}'");
            eprintln!("  usage: sammy-cli corpus validate <package.json>");
            ExitCode::from(2)
        }
        None => {
            eprintln!("sammy-cli corpus: missing subcommand");
            eprintln!("  usage: sammy-cli corpus validate <package.json>");
            ExitCode::from(2)
        }
    }
}
