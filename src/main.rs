//! wsx: keeps personal files off shared branches while moving work between a
//! workspace branch and its worktrees.
//!
//! The command-line surface grows with later phases' requirements; today it
//! only reports its version. Exit codes become a versioned contract in
//! Phase 5 (IF-01): until then, 0 is success and 2 is a usage error.

use std::process::ExitCode;

/// Exit code for input the CLI does not accept.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--version" => {
            println!("wsx {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: wsx --version");
            ExitCode::from(EXIT_USAGE)
        }
    }
}
