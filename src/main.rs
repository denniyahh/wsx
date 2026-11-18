//! wsx: keeps personal files off shared branches while moving work between a
//! workspace branch and its worktrees.
//!
//! The command-line surface grows with later phases' requirements; `cli`
//! parses it against one command table. Exit codes are a versioned contract:
//! docs/exit-codes.md.

use std::process::ExitCode;

mod check;
mod cli;
mod exit;
mod git;
mod json;
mod model;

fn main() -> ExitCode {
    cli::run(std::env::args_os().skip(1).collect())
}
