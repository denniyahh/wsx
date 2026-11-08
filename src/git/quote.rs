//! Quoting bytes for output: `c_quote` for display, the one gix quoting call
//! wsx makes, and `shell_word` for an argument in a command wsx prints for
//! the user to run.

use gix::bstr::ByteSlice;

/// `raw` in git's C-style quoting, as git prints a path: printable ASCII
/// that needs no quoting comes back unchanged, and anything else is wrapped
/// in double quotes with control and non-ASCII bytes as escapes (`\033` for
/// ESC). Research measured it equal to `git ls-files` on every byte value.
pub fn c_quote(raw: &[u8]) -> Vec<u8> {
    gix::quote::ansi_c::quote(raw.as_bstr()).to_vec()
}

/// `raw` as one word a shell reads back as exactly `raw`.
pub fn shell_word(raw: &[u8]) -> Option<String> {
    Some(String::from_utf8_lossy(raw).into_owned())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use super::super::testing::{self, Scratch, T0, commit_all, git_ok, write};
    use super::{c_quote, shell_word};

    /// A ref name `git check-ref-format` accepts that holds a command
    /// substitution creating the file `marker` in the shell's directory.
    const SHELL_SYNTAX_REF: &str = "refs/replace/$(touch${IFS}marker)";

    /// `<root>/<name>` with `a.txt` committed on `main` and the ref
    /// `SHELL_SYNTAX_REF` at `HEAD`, set by git with the name as one
    /// argument; canonical.
    fn repo_with_shell_syntax_ref(scratch: &Scratch, name: &str) -> PathBuf {
        let dir = scratch.root.join(name);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|err| panic!("create {}: {err}", dir.display()));
        let ok = |args: &[&str]| git_ok(scratch, &dir, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["init", "-q", "-b", "main"]);
        write(&dir.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &dir, "first", T0).expect("commit");
        ok(&["update-ref", SHELL_SYNTAX_REF, "HEAD"]);
        assert!(
            ref_exists(scratch, &dir),
            "git update-ref did not create {SHELL_SYNTAX_REF}"
        );
        dir.canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", dir.display()))
    }

    /// Whether `git show-ref --verify --quiet SHELL_SYNTAX_REF` finds the ref
    /// in `dir`.
    fn ref_exists(scratch: &Scratch, dir: &Path) -> bool {
        let output = testing::git(
            scratch,
            dir,
            ["show-ref", "--verify", "--quiet", SHELL_SYNTAX_REF],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        match output.status.code() {
            Some(0) => true,
            Some(1) => false,
            other => panic!(
                "git show-ref --verify --quiet {SHELL_SYNTAX_REF} exited {other:?}: {}",
                String::from_utf8_lossy(&output.stderr).trim_end()
            ),
        }
    }

    /// Runs `line` as `/bin/sh -c <line>` in `dir` with the scratch
    /// environment and stdin from `/dev/null`.
    fn run_pasted(scratch: &Scratch, dir: &Path, line: &str) -> Output {
        testing::command("/bin/sh", dir, &scratch.env)
            .arg("-c")
            .arg(line)
            .output()
            .unwrap_or_else(|err| panic!("run /bin/sh -c {line:?}: {err}"))
    }

    #[test]
    fn shell_word_quotes_what_a_shell_would_read() {
        let rows: [(&[u8], &str); 7] = [
            (b"origin", "origin"),
            (b"a/b", "a/b"),
            (
                b"refs/remotes/origin/develop",
                "refs/remotes/origin/develop",
            ),
            (
                b"refs/replace/$(touch${IFS}m)",
                "'refs/replace/$(touch${IFS}m)'",
            ),
            (b"it's", r"'it'\''s'"),
            (b"a;b", "'a;b'"),
            (b"a|b", "'a|b'"),
        ];
        for (raw, word) in rows {
            assert_eq!(
                shell_word(raw).as_deref(),
                Some(word),
                "shell_word({:?})",
                String::from_utf8_lossy(raw)
            );
        }
    }

    #[test]
    fn shell_word_has_no_form_for_bytes_a_quote_cannot_carry() {
        let rows: [&[u8]; 5] = [b"", b" ", b"\\", b"\x1b", b"\xe9"];
        for raw in rows {
            assert_eq!(shell_word(raw), None, "shell_word({raw:?})");
        }
    }

    #[test]
    fn pasted_command_removes_a_ref_named_with_shell_syntax() {
        let scratch = Scratch::new("shell-word-paste").expect("create scratch");
        let dir = repo_with_shell_syntax_ref(&scratch, "repo");
        let word = shell_word(SHELL_SYNTAX_REF.as_bytes())
            .unwrap_or_else(|| panic!("shell_word({SHELL_SYNTAX_REF}) has no form"));
        let line = format!("git update-ref --no-deref -d {word}");
        let output = run_pasted(&scratch, &dir, &line);
        assert_eq!(
            output.status.code(),
            Some(0),
            "/bin/sh -c {line:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
        assert!(
            !dir.join("marker").exists(),
            "/bin/sh -c {line:?} ran the command substitution"
        );
        assert!(
            !ref_exists(&scratch, &dir),
            "/bin/sh -c {line:?} left {SHELL_SYNTAX_REF}"
        );
    }

    #[test]
    fn display_quoting_leaves_a_command_substitution_live() {
        let scratch = Scratch::new("c-quote-paste").expect("create scratch");
        let dir = repo_with_shell_syntax_ref(&scratch, "repo");
        let shown = String::from_utf8(c_quote(SHELL_SYNTAX_REF.as_bytes()))
            .expect("c_quote of ASCII is ASCII");
        let line = format!("git update-ref --no-deref -d {shown}");
        let output = run_pasted(&scratch, &dir, &line);
        assert_ne!(
            output.status.code(),
            Some(0),
            "/bin/sh -c {line:?} succeeded"
        );
        assert!(
            dir.join("marker").exists(),
            "/bin/sh -c {line:?} did not run the command substitution"
        );
        assert!(
            ref_exists(&scratch, &dir),
            "/bin/sh -c {line:?} removed {SHELL_SYNTAX_REF}"
        );
    }
}
