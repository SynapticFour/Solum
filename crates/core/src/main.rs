//! Minimal Solum entrypoint: load a jurisdiction profile and validate runtime config.
//!
//! Usage:
//!   solum check --profile config/profiles/eu-ehds.toml
//!   SOLUM_STORAGE_REGION=us-east-1 solum check --profile config/profiles/eu-ehds.toml

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use solum_core::{example_eu_runtime, start_with_profile};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "check".into());
    if cmd != "check" {
        eprintln!("usage: solum check [--profile <path>]");
        return ExitCode::from(2);
    }

    let mut profile = PathBuf::from("config/profiles/eu-ehds.toml");
    while let Some(arg) = args.next() {
        if arg == "--profile" {
            if let Some(p) = args.next() {
                profile = PathBuf::from(p);
            } else {
                eprintln!("missing value for --profile");
                return ExitCode::from(2);
            }
        } else {
            eprintln!("unknown argument: {arg}");
            return ExitCode::from(2);
        }
    }

    let mut runtime = example_eu_runtime();
    if let Ok(region) = env::var("SOLUM_STORAGE_REGION") {
        runtime.storage_region = region;
    }

    match start_with_profile(&profile, &runtime) {
        Ok(p) => {
            println!(
                "ok: profile '{}' (jurisdiction {}) matches runtime configuration",
                p.meta.profile, p.meta.jurisdiction
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("fatal: {e}");
            ExitCode::FAILURE
        }
    }
}
