//! The command table: every command `wsx` accepts, as data only.
//!
//! Nothing here names another module of the crate, so a test crate can
//! include this file with `#[path]` and read the same table the binary
//! parses argv against: the contract tests compare it with the documented
//! commands and their schemas.

/// Which command a `CommandSpec` names; the dispatcher matches on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    /// `wsx check`: the divergence check (DIV-01).
    Check,
}

/// One command: its id, the word that runs it and its JSON schema version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    /// What the dispatcher runs.
    pub id: CommandId,
    /// The argv word that selects the command.
    pub name: &'static str,
    /// The version of the command's JSON output schema.
    pub schema_version: u32,
}

/// Every command, in the order usage lists them.
pub const COMMANDS: &[CommandSpec] = &[CommandSpec {
    id: CommandId::Check,
    name: "check",
    schema_version: 1,
}];
