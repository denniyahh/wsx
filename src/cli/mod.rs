//! The command line: parses argv against the command table, runs the
//! command, renders its output and errors, and turns the result into the
//! exit code (D-06, `docs/exit-codes.md`).
//!
//! Grammar: `wsx --version`, alone, or `wsx <name>` for a name in
//! `commands::COMMANDS`. Anything else is a usage error: nothing on stdout,
//! the problem and the usage lines on stderr, exit 2. wsx never prompts and
//! never reads stdin here.

pub mod commands;

use std::ffi::OsString;
use std::process::ExitCode;

use crate::check::{self, report};
use crate::exit::{self, ExitClass};
use crate::git::{self, Git, Repo};
use crate::model;

use commands::{COMMANDS, CommandId, CommandSpec};

/// Runs wsx with `args`, the arguments after the program name.
pub fn run(args: Vec<OsString>) -> ExitCode {
    match parse(&args) {
        Ok(Parsed::Version) => {
            println!("wsx {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Parsed::Command(spec)) => match spec.id {
            CommandId::Check => run_check(spec),
        },
        Err(problem) => {
            eprintln!("wsx: {problem}");
            eprintln!("usage: wsx --version");
            for spec in COMMANDS {
                eprintln!("usage: wsx {}", spec.name);
            }
            ExitCode::from(ExitClass::Usage.code())
        }
    }
}

/// What argv asks for.
enum Parsed {
    /// `wsx --version`.
    Version,
    /// A command from the table.
    Command(&'static CommandSpec),
}

/// `arg` for a usage message, C-quoted so no control byte reaches the
/// terminal raw.
fn shown(arg: &str) -> String {
    String::from_utf8_lossy(&git::c_quote(arg.as_bytes())).into_owned()
}

/// Parses `args` per the grammar, or says why they are not accepted.
fn parse(args: &[OsString]) -> Result<Parsed, String> {
    let words = args
        .iter()
        .map(|arg| arg.to_str())
        .collect::<Option<Vec<&str>>>()
        .ok_or_else(|| "an argument is not valid UTF-8".to_owned())?;
    match words.as_slice() {
        [] => Err("no command given".to_owned()),
        ["--version"] => Ok(Parsed::Version),
        [word, rest @ ..] => {
            let spec = COMMANDS
                .iter()
                .find(|spec| spec.name == *word)
                .ok_or_else(|| format!("unknown command {}", shown(word)))?;
            match rest {
                [] => Ok(Parsed::Command(spec)),
                [extra, ..] => Err(format!(
                    "{} takes no argument, got {}",
                    spec.name,
                    shown(extra)
                )),
            }
        }
    }
}

/// Runs `wsx check` in the current directory's repository: diagnostics to
/// stderr, the verdict to stdout, an error to stderr with its class's code.
fn run_check(spec: &CommandSpec) -> ExitCode {
    match check_here() {
        Ok(report) => {
            let human = report::human(&report);
            for line in &human.diagnostics {
                eprintln!("{line}");
            }
            println!("{}", human.verdict);
            let class = if report.problems.is_empty() {
                ExitClass::Success
            } else {
                ExitClass::Problems
            };
            ExitCode::from(class.code())
        }
        Err(err) => {
            eprintln!("wsx {}: {err}", spec.name);
            ExitCode::from(exit::classify(&err).class.code())
        }
    }
}

/// The check's report for the repository containing the current directory.
fn check_here() -> Result<check::Report, model::Error> {
    let cwd = std::env::current_dir().map_err(|source| git::Error::Io {
        what: "read the current directory".to_owned(),
        source,
    })?;
    let repo = Repo::discover(&cwd)?;
    let git = Git::from_process(&repo);
    check::run(&repo, &git)
}
