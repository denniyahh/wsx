//! The black-box scenario harness: one integration-test target, `wsx::harness`.
//!
//! Each scenario names an abstract action, builds a fixture repository and
//! states its expected outcome explicitly. Its oracle side runs devflow's
//! vendored shell scripts from a patched scratch copy; its Rust side runs the
//! `wsx` binary, and stays an ignored test until the phase that implements the
//! action enables it. A test is named `<module>::<scenario>::<side>`, e.g.
//! `sync::self_test::oracle`.
//!
//! Fixtures are isolated per scenario: a `tempfile::TempDir` under
//! `CARGO_TARGET_TMPDIR` named `harness-<module>-<name>-`, and every spawned
//! process starts from an empty environment holding only the harness keys
//! (scenario HOME, XDG dirs and git config, a fixed PATH, a git ceiling at the
//! scratch root) plus `LLVM_*`. Nothing reads the host's git config, hooks or
//! identity, and nothing discovers the wsx checkout around `target/`.
//!
//! Every module is declared private, with a plain `mod`, so `clippy -D
//! warnings` rejects a helper nothing uses.

#[macro_use]
mod scenario;

mod action;
mod coverage;
mod env;
mod fixture;
mod hermeticity;
mod oracle;
mod outcome;
mod provenance;
mod registry;
mod rust;

/// Declares each scenario module and concatenates the registries they define
/// with `scenarios!`, so a module cannot be declared without being registered.
macro_rules! harness_modules {
    ($($module:ident),+ $(,)?) => {
        $(mod $module;)+

        /// Each scenario module's name with the scenarios it registers.
        fn registries() -> Vec<(&'static str, Vec<scenario::Entry>)> {
            vec![$((stringify!($module), $module::registry())),+]
        }

        /// Every registered scenario.
        fn all_scenarios() -> Vec<scenario::Entry> {
            registries()
                .into_iter()
                .flat_map(|(_, entries)| entries)
                .collect()
        }
    };
}

harness_modules!(sync);
