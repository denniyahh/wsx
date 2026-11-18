//! The command line: parses argv against the command table, runs the
//! command, renders its output and errors, and turns the result into the
//! exit code (D-06, `docs/exit-codes.md`).
//!
//! Grammar: `wsx --version`, alone, or `wsx <name> [--json]` for a name in
//! `commands::COMMANDS`, `--json` at most once. Anything else is a usage
//! error, exit 2: the problem and the usage lines on stderr, and nothing on
//! stdout, unless the first argument names a command and `--json` is among
//! the arguments after it, when stdout carries that command's failure object
//! (D-08). wsx never prompts and never reads stdin here.
//!
//! `Sinks` decides where each line goes: diagnostics to stderr, the verdict
//! to stdout, or to stderr with `--json`, and the JSON object to stdout,
//! once, last.

pub mod commands;

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::process::ExitCode;

use crate::check::{self, report};
use crate::exit::{self, ExitClass};
use crate::git::{self, Git, Repo};
use crate::json::Value;
use crate::model;

use commands::{COMMANDS, CommandId, CommandSpec};

/// The flag that asks for the JSON object on stdout.
const JSON_FLAG: &str = "--json";

/// Runs wsx with `args`, the arguments after the program name.
pub fn run(args: Vec<OsString>) -> ExitCode {
    match parse(&args) {
        Ok(Parsed::Version) => {
            println!("wsx {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Parsed::Command { spec, json }) => {
            let sinks = Sinks { json };
            match spec.id {
                CommandId::Check => run_check(spec, &sinks),
            }
        }
        Err(usage) => usage_error(&usage),
    }
}

/// What argv asks for.
enum Parsed {
    /// `wsx --version`.
    Version,
    /// A command from the table, with or without `--json`.
    Command {
        spec: &'static CommandSpec,
        json: bool,
    },
}

/// Why argv is not accepted.
struct Usage {
    /// The problem, for the message.
    problem: String,
    /// The command whose failure object stdout carries: set when the first
    /// argument names a command and `--json` follows it.
    json_for: Option<&'static CommandSpec>,
}

/// `arg` for a usage message, C-quoted so no control byte reaches the
/// terminal raw; C-quoting leaves only ASCII, so nothing is lost.
fn shown(arg: &OsStr) -> String {
    String::from_utf8_lossy(&git::c_quote(arg.as_encoded_bytes())).into_owned()
}

/// Parses `args` per the grammar, or says why they are not accepted.
fn parse(args: &[OsString]) -> Result<Parsed, Usage> {
    let plain = |problem: String| Usage {
        problem,
        json_for: None,
    };
    let Some((first, rest)) = args.split_first() else {
        return Err(plain("no command given".to_owned()));
    };
    if first == "--version" && rest.is_empty() {
        return Ok(Parsed::Version);
    }
    let spec = COMMANDS
        .iter()
        .find(|spec| first == spec.name)
        .ok_or_else(|| plain(format!("unknown command {}", shown(first))))?;
    let json = rest.iter().any(|arg| arg == JSON_FLAG);
    let refused = |problem: String| Usage {
        problem,
        json_for: json.then_some(spec),
    };
    let mut seen = false;
    for arg in rest {
        if arg != JSON_FLAG {
            return Err(refused(format!(
                "{} takes no argument but {JSON_FLAG}, got {}",
                spec.name,
                shown(arg)
            )));
        }
        if seen {
            return Err(refused(format!("{JSON_FLAG} is given more than once")));
        }
        seen = true;
    }
    Ok(Parsed::Command { spec, json })
}

/// Reports `usage`: the problem and the usage lines on stderr, then, when
/// `--json` was asked for after a command, its failure object; exit 2.
fn usage_error(usage: &Usage) -> ExitCode {
    eprintln!("wsx: {}", usage.problem);
    eprintln!("usage: wsx --version");
    for spec in COMMANDS {
        eprintln!("usage: wsx {} [{JSON_FLAG}]", spec.name);
    }
    let class = ExitClass::Usage;
    match usage.json_for {
        Some(spec) => Sinks { json: true }.finish(class, || {
            failure(spec, "usage", usage.problem.clone(), class)
        }),
        None => ExitCode::from(class.code()),
    }
}

/// Where each line of a run goes (D-08).
struct Sinks {
    /// Whether the run prints its JSON object.
    json: bool,
}

impl Sinks {
    /// A diagnostic line: stderr.
    fn diagnostic(&self, line: &str) {
        eprintln!("{line}");
    }

    /// The verdict line: stdout, or stderr when stdout carries the object.
    fn verdict(&self, line: &str) {
        if self.json {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
    }

    /// Ends the run as `class`; with `--json`, first writes `object`'s
    /// rendering and a newline to stdout in one write. A failed write is
    /// reported on stderr and exits as the operating-system failure it is.
    fn finish(&self, class: ExitClass, object: impl FnOnce() -> Value) -> ExitCode {
        if self.json {
            let mut rendered = object().render();
            rendered.push('\n');
            let mut stdout = std::io::stdout().lock();
            if let Err(err) = stdout
                .write_all(rendered.as_bytes())
                .and_then(|()| stdout.flush())
            {
                eprintln!("wsx: cannot write the JSON object to stdout: {err}");
                return ExitCode::from(ExitClass::GitFailed.code());
            }
        }
        ExitCode::from(class.code())
    }
}

/// The object every `--json` run prints: the command's name and schema
/// version from its `COMMANDS` entry, then `payload`.
fn envelope(spec: &CommandSpec, payload: Vec<(&'static str, Value)>) -> Value {
    let mut members = vec![
        ("command", Value::Str(spec.name.to_owned())),
        ("schema_version", Value::Int(i64::from(spec.schema_version))),
    ];
    members.extend(payload);
    Value::Object(members)
}

/// `spec`'s failure object: `error` with the error's `kind`, its `message`
/// and the exit code of `class`.
fn failure(spec: &CommandSpec, kind: &str, message: String, class: ExitClass) -> Value {
    envelope(
        spec,
        vec![(
            "error",
            Value::Object(vec![
                ("kind", Value::Str(kind.to_owned())),
                ("message", Value::Str(message)),
                ("exit_code", Value::Int(i64::from(class.code()))),
            ]),
        )],
    )
}

/// Runs `wsx check` in the current directory's repository: diagnostics and
/// errors to stderr, the verdict through `sinks`, then the object with
/// `--json`; exits with the outcome's class.
fn run_check(spec: &CommandSpec, sinks: &Sinks) -> ExitCode {
    match check_here() {
        Ok(report) => {
            let human = report::human(&report);
            for line in &human.diagnostics {
                sinks.diagnostic(line);
            }
            sinks.verdict(&human.verdict);
            let class = if report.problems.is_empty() {
                ExitClass::Success
            } else {
                ExitClass::Problems
            };
            sinks.finish(class, || envelope(spec, report::json(&report)))
        }
        Err(err) => {
            sinks.diagnostic(&format!("wsx {}: {err}", spec.name));
            let classified = exit::classify(&err);
            sinks.finish(classified.class, || {
                failure(spec, classified.kind, err.to_string(), classified.class)
            })
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
