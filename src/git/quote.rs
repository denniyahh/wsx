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

/// The raw bytes of `token`, a path as `c_quote` or git prints it. Not yet
/// implemented: returns `token` unchanged.
pub fn c_unquote(token: &[u8]) -> Result<Vec<u8>, String> {
    Ok(token.to_vec())
}

/// `raw` as one word that `sh`, `bash` and `fish` each read back as exactly
/// `raw`, for an argument in a command wsx prints for the user to paste, or
/// `None` when no such quoting exists here: for an empty value, and for any
/// byte outside printable ASCII (0x21 to 0x7e) or equal to `\`. A caller
/// prints a placeholder instead, so that command must be edited before it
/// runs. Otherwise the bytes come back bare when each is an ASCII letter, a
/// digit, `/`, `.`, `_` or `-`, and single-quoted, each `'` written `'\''`,
/// when any is not.
///
/// `c_quote` is display quoting and leaves `$`, `(` and backquote alone, so a
/// command line built from it runs a command substitution held in a ref
/// name: `git check-ref-format` accepts `refs/replace/$(touch${IFS}m)`.
/// Measured on git 2.56.0: with that name single-quoted, `sh -c`, `bash -c`
/// and `fish -c` each removed the ref with exit 0 and ran nothing, and
/// `'\''` removed `refs/replace/it's` in all three; with the name bare, `sh`
/// and `bash` ran the substitution and `git update-ref` exited 1 with the
/// ref still present. dash, the pinned image's `/bin/sh`, gave the same two
/// results. Backslash is excluded because fish reads `\\` and `\'` inside
/// single quotes and sh does not.
pub fn shell_word(raw: &[u8]) -> Option<String> {
    let printable = |byte: &u8| (0x21..=0x7e).contains(byte) && *byte != b'\\';
    if raw.is_empty() || !raw.iter().all(printable) {
        return None;
    }
    // Printable ASCII is valid UTF-8, so the lossy conversion loses nothing.
    let text = String::from_utf8_lossy(raw);
    let bare = |byte: &u8| byte.is_ascii_alphanumeric() || b"/._-".contains(byte);
    if raw.iter().all(bare) {
        Some(text.into_owned())
    } else {
        Some(format!("'{}'", text.replace('\'', r"'\''")))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use gix::bstr::ByteSlice;

    use super::super::testing::{
        self, Scratch, T0, all_bytes_repo, commit_all, git_ok, records, write,
    };
    use super::{c_quote, c_unquote, shell_word};

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

    /// The raw names `git ls-files -z` lists in `repo`, and the lines
    /// `git <config> ls-files` prints there, both in index order, so the two
    /// lists pair up by position.
    fn listing(scratch: &Scratch, repo: &Path, config: &[&str]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let ls = |args: &[&str]| git_ok(scratch, repo, args).unwrap_or_else(|err| panic!("{err}"));
        let raw = records(&ls(&["ls-files", "-z"]), 0);
        let lines = records(&ls(&[config, &["ls-files"]].concat()), b'\n');
        assert_eq!(
            raw.len(),
            lines.len(),
            "git {config:?} ls-files and git ls-files -z list different counts"
        );
        (raw, lines)
    }

    /// Each position where `ours` and `git` differ, both shown escaped.
    fn differences(ours: &[Vec<u8>], git: &[Vec<u8>]) -> Vec<String> {
        assert_eq!(ours.len(), git.len(), "the lists differ in length");
        ours.iter()
            .zip(git)
            .enumerate()
            .filter(|(_, (left, right))| left != right)
            .map(|(index, (left, right))| {
                format!(
                    "{index}: ours {:?}, git {:?}",
                    left.as_bstr(),
                    right.as_bstr()
                )
            })
            .collect()
    }

    /// How many positions of `ours` and `git` differ, with the first five,
    /// or `None` when the lists are equal.
    fn mismatch_report(ours: &[Vec<u8>], git: &[Vec<u8>]) -> Option<String> {
        const SHOWN: usize = 5;
        let found = differences(ours, git);
        (!found.is_empty()).then(|| {
            format!(
                "{} of {} differ, first {:?}",
                found.len(),
                ours.len(),
                &found[..found.len().min(SHOWN)]
            )
        })
    }

    #[test]
    fn c_quote_matches_git_on_every_byte() {
        let scratch = Scratch::new("quote-all-bytes").expect("create scratch");
        let repo = all_bytes_repo(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let (raw, lines) = listing(&scratch, &repo, &[]);
        let ours: Vec<Vec<u8>> = raw.iter().map(|name| c_quote(name)).collect();
        if let Some(report) = mismatch_report(&ours, &lines) {
            panic!("c_quote differs from git ls-files: {report}");
        }
    }

    #[test]
    fn a_changed_byte_is_detected() {
        let scratch = Scratch::new("quote-changed-byte").expect("create scratch");
        let repo = all_bytes_repo(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let (mut raw, lines) = listing(&scratch, &repo, &[]);
        let changed = raw
            .iter()
            .position(|name| name == b"p\x01q")
            .expect("git ls-files -z lists p<01>q");
        raw[changed][1] = 0x02;
        let ours: Vec<Vec<u8>> = raw.iter().map(|name| c_quote(name)).collect();
        assert_eq!(
            differences(&ours, &lines).len(),
            1,
            "one changed byte did not make exactly one quoted line differ"
        );
    }

    #[test]
    fn c_unquote_reverses_git_output_on_every_byte() {
        let scratch = Scratch::new("unquote-all-bytes").expect("create scratch");
        let repo = all_bytes_repo(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let (raw, quoted) = listing(&scratch, &repo, &[]);
        let (raw_again, unquoted_path) = listing(&scratch, &repo, &["-c", "core.quotePath=false"]);
        assert_eq!(raw, raw_again, "git ls-files -z changed between runs");
        assert_ne!(
            quoted, unquoted_path,
            "core.quotePath=false printed the same lines as the default"
        );
        for (config, lines) in [
            ("default", &quoted),
            ("core.quotePath=false", &unquoted_path),
        ] {
            let ours: Vec<Vec<u8>> = lines
                .iter()
                .map(|line| {
                    c_unquote(line)
                        .unwrap_or_else(|err| panic!("c_unquote({:?}): {err}", line.as_bstr()))
                })
                .collect();
            if let Some(report) = mismatch_report(&ours, &raw) {
                panic!(
                    "c_unquote of git ls-files ({config}) differs from git ls-files -z: {report}"
                );
            }
        }
    }

    #[test]
    fn octal_escape_at_the_byte_limit_parses() {
        assert_eq!(c_unquote(br#""\377""#), Ok(vec![0xff]));
    }

    #[test]
    fn octal_escape_past_the_byte_limit_is_refused() {
        let result = c_unquote(br#""\400""#);
        assert!(
            result.is_err(),
            "c_unquote of an escape past 377: {result:?}"
        );
    }

    #[test]
    fn c_unquote_refuses_text_after_the_closing_quote() {
        assert_eq!(
            c_unquote(br#""a"b"#),
            Err("text after the closing quote".to_owned())
        );
    }

    #[test]
    fn c_unquote_refuses_an_unterminated_token() {
        let result = c_unquote(br#""abc"#);
        assert!(
            result.is_err(),
            "c_unquote of an unterminated token: {result:?}"
        );
    }

    #[test]
    fn c_unquote_refuses_an_unknown_escape() {
        let result = c_unquote(br#""a\q""#);
        assert!(
            result.is_err(),
            "c_unquote of an unknown escape: {result:?}"
        );
    }
}
