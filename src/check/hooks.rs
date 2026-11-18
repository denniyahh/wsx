//! The hook-mode rule (DIV-03, D-03): every file under git's effective
//! `core.hooksPath` must be committed executable in the workspace tip's tree.
//!
//! git runs only executable hooks, and a checkout with `core.fileMode=false`
//! records a new file as 100644, so a hook committed that way never runs on a
//! fresh clone however it looks on disk. The modes therefore come from the
//! commit's tree listing, never the index or the working tree. Paths outside
//! the hooks directory, `scripts/*.sh` among them, are not judged.
//!
//! A 120000 entry passes when its target resolves, inside the committed tree,
//! to a 100755 blob. Resolution walks the target as the kernel walks a path:
//! component by component from the link's parent, `..` popping what has been
//! resolved, and any component that is itself a symlink, a directory link in
//! the middle of the path included, has its own target spliced in from that
//! link's parent. Every link followed counts against one budget of
//! `SYMLINK_HOPS`, so a loop through file or directory links ends. Leaving the
//! root, an absolute target, an untracked or non-directory component, or an
//! exhausted budget fails, naming the hook; nothing unresolved passes.

use std::collections::{BTreeMap, VecDeque};

use super::{Problem, ProblemKind};
use crate::git::{ListedEntry, MODE_EXECUTABLE, MODE_SYMLINK, MODE_TREE, ObjectId, c_quote};
use crate::model::manifest::normalize_path;

/// The most symlinks one resolution follows, Linux's `MAXSYMLINKS`.
pub const SYMLINK_HOPS: usize = 40;

/// Why an absolute `core.hooksPath` is not checked.
const ABSOLUTE_REASON: &str = "it is an absolute path, outside the commits wsx reads";

/// Why a relative `core.hooksPath` that names no tree path is not checked.
const OUTSIDE_REASON: &str = "it is not a relative path inside the repository";

/// What the check made of `core.hooksPath`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HookStatus {
    /// git has no value for it.
    NotSet,
    /// A value wsx does not check, and why.
    NotChecked {
        /// The value as git gave it, raw bytes.
        value: Vec<u8>,
        /// Why it is not checked.
        reason: String,
    },
    /// The directory, from the tree root, whose hooks were judged.
    Checked {
        /// The normalised directory path.
        dir: Vec<u8>,
    },
}

/// The tree directory `value` names, or why it is not checked.
pub fn normalise(value: &[u8]) -> Result<Vec<u8>, HookStatus> {
    Ok(value.to_vec())
}

/// `Checked` when `listing`, the tip of `branch`, holds a directory at `dir`.
pub fn at_tip(_value: &[u8], dir: Vec<u8>, _listing: &[ListedEntry], _branch: &[u8]) -> HookStatus {
    HookStatus::Checked { dir }
}

/// The hook problems under `dir` in `listing`.
pub fn evaluate_hooks(
    _dir: &[u8],
    _listing: &[ListedEntry],
    _targets: &BTreeMap<ObjectId, Vec<u8>>,
) -> Vec<Problem> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// git's tree entry mode of a regular file.
    const PLAIN: u32 = 0o100644;
    /// git's tree entry mode of a gitlink.
    const GITLINK: u32 = 0o160000;
    /// The hooks directory every judging test checks.
    const HOOKS: &[u8] = b"scripts/hooks";

    /// One entry of a hand-built tree: its mode, its path and, for a
    /// symlink, the link text its blob holds.
    type Item<'a> = (u32, &'a [u8], Option<&'a [u8]>);

    /// A file of `mode` at `path`.
    fn file(mode: u32, path: &[u8]) -> Item<'_> {
        (mode, path, None)
    }

    /// A symlink at `path` whose link text is `target`.
    fn link<'a>(path: &'a [u8], target: &'a [u8]) -> Item<'a> {
        (MODE_SYMLINK, path, Some(target))
    }

    /// A directory at `path`.
    fn dir(path: &[u8]) -> Item<'_> {
        (MODE_TREE, path, None)
    }

    /// The listing of a tree holding `items` and every directory above
    /// them, in path order, with an id of its own for each entry, and the
    /// link text of each symlink by its id.
    fn tree(items: &[Item<'_>]) -> (Vec<ListedEntry>, BTreeMap<ObjectId, Vec<u8>>) {
        let mut by_path: BTreeMap<Vec<u8>, (u32, Option<Vec<u8>>)> = BTreeMap::new();
        for (mode, path, text) in items {
            for (index, byte) in path.iter().enumerate() {
                if *byte == b'/' {
                    by_path
                        .entry(path[..index].to_vec())
                        .or_insert((MODE_TREE, None));
                }
            }
            by_path.insert(path.to_vec(), (*mode, text.map(<[u8]>::to_vec)));
        }
        let mut listing = Vec::new();
        let mut targets = BTreeMap::new();
        for (index, (path, (mode, text))) in by_path.into_iter().enumerate() {
            let id = ObjectId::for_test(&format!("{:040x}", index + 1));
            if let Some(text) = text {
                targets.insert(id.clone(), text);
            }
            listing.push(ListedEntry { mode, id, path });
        }
        (listing, targets)
    }

    /// The kind, path and detail of each problem `evaluate_hooks` finds
    /// under `scripts/hooks` in a tree holding `items`.
    fn judge(items: &[Item<'_>]) -> Vec<(&'static str, String, String)> {
        let (listing, targets) = tree(items);
        found(&evaluate_hooks(HOOKS, &listing, &targets))
    }

    /// The kind, path and detail of each of `problems`.
    fn found(problems: &[Problem]) -> Vec<(&'static str, String, String)> {
        problems
            .iter()
            .map(|problem| {
                (
                    problem.kind.name(),
                    String::from_utf8_lossy(&problem.path).into_owned(),
                    problem.detail.clone().unwrap_or_default(),
                )
            })
            .collect()
    }

    /// The one problem a hook at `path` with `detail` is.
    fn hook_problem(path: &str, detail: &str) -> Vec<(&'static str, String, String)> {
        vec![("hook_not_executable", path.to_owned(), detail.to_owned())]
    }

    /// The detail of a symlink hook that does not resolve, for `reason`.
    fn unresolved(reason: &str) -> String {
        format!("a symlink that does not resolve to a tracked executable: {reason}")
    }

    /// The status `normalise` refuses `value` with, for `reason`.
    fn not_checked(value: &[u8], reason: &str) -> HookStatus {
        HookStatus::NotChecked {
            value: value.to_vec(),
            reason: reason.to_owned(),
        }
    }

    #[test]
    fn relative_hooks_path_is_normalised() {
        assert_eq!(normalise(b"scripts/hooks"), Ok(HOOKS.to_vec()));
        assert_eq!(normalise(b"./scripts/hooks/"), Ok(HOOKS.to_vec()));
    }

    #[test]
    fn absolute_hooks_path_is_not_checked() {
        assert_eq!(
            normalise(b"/home/ada/hooks"),
            Err(not_checked(b"/home/ada/hooks", ABSOLUTE_REASON))
        );
        assert_eq!(
            ABSOLUTE_REASON,
            "it is an absolute path, outside the commits wsx reads"
        );
    }

    #[test]
    fn hooks_path_with_a_dot_component_is_not_checked() {
        assert_eq!(
            normalise(b"../hooks"),
            Err(not_checked(b"../hooks", OUTSIDE_REASON))
        );
        assert_eq!(
            normalise(b"a/./b"),
            Err(not_checked(b"a/./b", OUTSIDE_REASON))
        );
        assert_eq!(
            OUTSIDE_REASON,
            "it is not a relative path inside the repository"
        );
    }

    #[test]
    fn empty_hooks_path_is_not_checked() {
        assert_eq!(normalise(b""), Err(not_checked(b"", OUTSIDE_REASON)));
        assert_eq!(normalise(b"./"), Err(not_checked(b"./", OUTSIDE_REASON)));
    }

    /// The directory must be a tree at the tip: absent, or a blob, it is not
    /// checked and the reason names it; a tree is checked.
    #[test]
    fn hooks_dir_must_be_a_tree_at_the_tip() {
        let reason = "scripts/hooks is not a directory at the tip of workspace/dev";
        let (absent, _) = tree(&[file(MODE_EXECUTABLE, b"scripts/run")]);
        assert_eq!(
            at_tip(
                b"./scripts/hooks",
                HOOKS.to_vec(),
                &absent,
                b"workspace/dev"
            ),
            not_checked(b"./scripts/hooks", reason)
        );
        let (blob, _) = tree(&[file(MODE_EXECUTABLE, HOOKS)]);
        assert_eq!(
            at_tip(b"scripts/hooks", HOOKS.to_vec(), &blob, b"workspace/dev"),
            not_checked(b"scripts/hooks", reason)
        );
        let (present, _) = tree(&[file(MODE_EXECUTABLE, b"scripts/hooks/pre-push")]);
        assert_eq!(
            at_tip(b"scripts/hooks", HOOKS.to_vec(), &present, b"workspace/dev"),
            HookStatus::Checked {
                dir: HOOKS.to_vec()
            }
        );
    }

    #[test]
    fn executable_hook_passes() {
        assert_eq!(
            judge(&[file(MODE_EXECUTABLE, b"scripts/hooks/pre-push")]),
            []
        );
    }

    #[test]
    fn plain_hook_fails_naming_its_mode() {
        assert_eq!(
            judge(&[file(PLAIN, b"scripts/hooks/pre-push")]),
            hook_problem(
                "scripts/hooks/pre-push",
                "committed 100644; commit it with mode 100755"
            )
        );
    }

    #[test]
    fn gitlink_in_the_hooks_dir_fails() {
        assert_eq!(
            judge(&[file(GITLINK, b"scripts/hooks/sub")]),
            hook_problem(
                "scripts/hooks/sub",
                "committed 160000; commit it with mode 100755"
            )
        );
    }

    #[test]
    fn symlink_to_an_executable_passes() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../bin/a"),
                file(MODE_EXECUTABLE, b"scripts/bin/a"),
            ]),
            []
        );
    }

    #[test]
    fn symlink_chain_to_an_executable_passes() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../l1"),
                link(b"scripts/l1", b"l2"),
                link(b"scripts/l2", b"bin/a"),
                file(MODE_EXECUTABLE, b"scripts/bin/a"),
            ]),
            []
        );
    }

    #[test]
    fn symlink_to_a_plain_file_fails() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../bin/a"),
                file(PLAIN, b"scripts/bin/a"),
            ]),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("its target scripts/bin/a is committed 100644")
            )
        );
    }

    #[test]
    fn dangling_symlink_fails() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../bin/a"),
                file(MODE_EXECUTABLE, b"scripts/bin/b"),
            ]),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("its target scripts/bin/a is not tracked")
            )
        );
    }

    /// From `scripts/hooks`, `../../` is the root, so `../../x` names the
    /// root's `x` and passes when it is executable (control); one more
    /// `../` leaves the tree and fails.
    #[test]
    fn symlink_leaving_the_tree_fails() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../../x"),
                file(MODE_EXECUTABLE, b"x"),
            ]),
            []
        );
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../../../x"),
                file(MODE_EXECUTABLE, b"x"),
            ]),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("its target ../../../x leaves the tree")
            )
        );
    }

    #[test]
    fn absolute_symlink_fails() {
        assert_eq!(
            judge(&[link(b"scripts/hooks/a", b"/usr/bin/true")]),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("its target /usr/bin/true is absolute")
            )
        );
    }

    #[test]
    fn symlink_to_a_directory_fails() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../bin"),
                file(MODE_EXECUTABLE, b"scripts/bin/a"),
            ]),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("its target scripts/bin is a directory")
            )
        );
    }

    #[test]
    fn symlink_through_a_symlinked_directory_passes() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../lib/a"),
                link(b"scripts/lib", b"tools"),
                file(MODE_EXECUTABLE, b"scripts/tools/a"),
            ]),
            []
        );
    }

    /// `../lib/../run` with `scripts/lib` a link to `vendor/tools`: the
    /// `..` leaves the link's target, so the hook is `scripts/vendor/run`,
    /// not `scripts/run`, where a lexical reading lands. With the modes
    /// swapped (control) the same hook fails naming `scripts/vendor/run`.
    #[test]
    fn dot_dot_after_a_symlinked_directory_leaves_its_target() {
        let items = |vendor_mode, lexical_mode| {
            vec![
                link(b"scripts/hooks/a", b"../lib/../run"),
                link(b"scripts/lib", b"vendor/tools"),
                dir(b"scripts/vendor/tools"),
                file(vendor_mode, b"scripts/vendor/run"),
                file(lexical_mode, b"scripts/run"),
            ]
        };
        assert_eq!(judge(&items(MODE_EXECUTABLE, PLAIN)), []);
        assert_eq!(
            judge(&items(PLAIN, MODE_EXECUTABLE)),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("its target scripts/vendor/run is committed 100644")
            )
        );
    }

    #[test]
    fn symlink_through_a_file_fails() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../run/x"),
                file(MODE_EXECUTABLE, b"scripts/run"),
            ]),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("its target passes through scripts/run, which is not a directory")
            )
        );
    }

    /// The links of a chain of `count` symlinks from `scripts/hooks/a`
    /// (the first) through `scripts/chain/l1`.. to `scripts/bin/x`, an
    /// executable, as (path, link text) pairs.
    fn chain(count: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
        let path = |index: usize| {
            if index == 1 {
                b"scripts/hooks/a".to_vec()
            } else {
                format!("scripts/chain/l{}", index - 1).into_bytes()
            }
        };
        let target = |index: usize| {
            if index == count {
                b"../bin/x".to_vec()
            } else if index == 1 {
                b"../chain/l1".to_vec()
            } else {
                format!("l{index}").into_bytes()
            }
        };
        (1..=count)
            .map(|index| (path(index), target(index)))
            .collect()
    }

    /// The problems of the tree holding `links` and `scripts/bin/x`.
    fn judge_chain(links: &[(Vec<u8>, Vec<u8>)]) -> Vec<(&'static str, String, String)> {
        let mut items: Vec<Item<'_>> = links
            .iter()
            .map(|(path, target)| link(path, target))
            .collect();
        items.push(file(MODE_EXECUTABLE, b"scripts/bin/x"));
        judge(&items)
    }

    /// A loop fails with the hop-limit reason on each hook in it; a chain of
    /// exactly `SYMLINK_HOPS` links passes (control) and one link more
    /// fails.
    #[test]
    fn symlink_loop_stops_at_the_hop_limit() {
        let hops = unresolved("it is one of more than 40 symlinks in a row");
        let mut looped = hook_problem("scripts/hooks/a", &hops);
        looped.extend(hook_problem("scripts/hooks/b", &hops));
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"b"),
                link(b"scripts/hooks/b", b"a"),
            ]),
            looped
        );
        assert_eq!(SYMLINK_HOPS, 40);
        assert_eq!(judge_chain(&chain(SYMLINK_HOPS)), []);
        assert_eq!(
            judge_chain(&chain(SYMLINK_HOPS + 1)),
            hook_problem("scripts/hooks/a", &hops)
        );
    }

    #[test]
    fn directory_symlink_loop_fails_naming_the_hook() {
        assert_eq!(
            judge(&[
                link(b"scripts/hooks/a", b"../loop/a"),
                link(b"scripts/loop", b"loop"),
            ]),
            hook_problem(
                "scripts/hooks/a",
                &unresolved("it is one of more than 40 symlinks in a row")
            )
        );
    }

    /// Only paths strictly below the hooks directory are hooks: a plain file
    /// in `scripts/hooksX` is not, one in a subdirectory of the hooks
    /// directory is (control).
    #[test]
    fn path_beside_the_hooks_dir_is_not_a_hook() {
        assert_eq!(
            judge(&[
                file(MODE_EXECUTABLE, b"scripts/hooks/pre-push"),
                file(PLAIN, b"scripts/hooksX/a"),
            ]),
            []
        );
        assert_eq!(
            judge(&[
                file(MODE_EXECUTABLE, b"scripts/hooks/pre-push"),
                file(PLAIN, b"scripts/hooksX/a"),
                file(PLAIN, b"scripts/hooks/sub/c"),
            ]),
            hook_problem(
                "scripts/hooks/sub/c",
                "committed 100644; commit it with mode 100755"
            )
        );
    }

    #[test]
    fn empty_hooks_dir_has_no_problem() {
        assert_eq!(judge(&[dir(b"scripts/hooks/sub/deeper")]), []);
        assert_eq!(
            judge(&[file(MODE_EXECUTABLE, b"scripts/hooks/pre-push")]),
            []
        );
    }

    /// `scripts/lib.sh` committed 100644 is outside the hooks directory and
    /// not judged; the same mode inside it is (control).
    #[test]
    fn scripts_sh_outside_the_hooks_dir_is_not_checked() {
        assert_eq!(
            judge(&[
                file(PLAIN, b"scripts/lib.sh"),
                file(MODE_EXECUTABLE, b"scripts/hooks/pre-push"),
            ]),
            []
        );
        assert_eq!(
            judge(&[
                file(PLAIN, b"scripts/lib.sh"),
                file(PLAIN, b"scripts/hooks/lib.sh")
            ]),
            hook_problem(
                "scripts/hooks/lib.sh",
                "committed 100644; commit it with mode 100755"
            )
        );
    }

    /// Problems come out in path-byte order whatever order the listing
    /// holds them in.
    #[test]
    fn hook_problems_are_sorted_by_path() {
        let (mut listing, targets) = tree(&[
            file(PLAIN, b"scripts/hooks/a"),
            file(PLAIN, b"scripts/hooks/b/c"),
            file(PLAIN, b"scripts/hooks/z"),
        ]);
        listing.reverse();
        let paths: Vec<String> = found(&evaluate_hooks(HOOKS, &listing, &targets))
            .into_iter()
            .map(|(_, path, _)| path)
            .collect();
        assert_eq!(
            paths,
            ["scripts/hooks/a", "scripts/hooks/b/c", "scripts/hooks/z"]
        );
    }
}
