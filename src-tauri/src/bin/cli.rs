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
        Some("corpus") => {
            // Phase 6 implements the real validator.
            eprintln!("sammy-cli: 'corpus' is not implemented until Phase 6");
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("sammy-cli: unknown subcommand '{other}'");
            eprintln!("\n{HELP}");
            ExitCode::from(2)
        }
    }
}
