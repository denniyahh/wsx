//! Commit-signing scenarios (plan 02-08).
//!
//! Each scenario runs an action that creates commits in a repository
//! configured to sign them, and states which of the resulting commits must
//! carry a signature.
//!
//! Isolation: every scenario builds its fixture, and any signing key it
//! needs, in its own `harness-signing-<name>-` tree under
//! `CARGO_TARGET_TMPDIR`, in the hermetic environment.
//!
//! Each scenario reuses a registered case, looked up in `all_scenarios()`,
//! declares signing on its repo config and changes only the `signed` field of
//! the commits the action creates, in its expected outcome and in any oracle
//! outcome it records. Signature presence is asserted, not validity.

use crate::outcome::{CommitExpect, Outcome, RefsExpect, Tip};
use crate::scenario::{Case, OracleSide, Role, Scenario};

/// The merge commit the sync creates.
const SYNC_MERGE: &[u8] = b"chore: sync develop into workspace/dev\n";
/// The commit the sync fetches; it is made before any key exists.
const FETCHED: &[u8] = b"feat: add b\n";
/// The commit the PR cut replays onto the PR branch.
const REPLAYED: &[u8] = b"feat(1): add b\n";

/// The registered scenario `module::name`, built.
fn registered(module: &str, name: &str) -> Scenario {
    let entry = crate::all_scenarios()
        .into_iter()
        .find(|entry| entry.module == module && entry.name == name)
        .unwrap_or_else(|| panic!("{module}::{name} is not registered"));
    (entry.build)()
}

/// Whether every expected commit with exactly this message is signed.
struct Signature {
    message: &'static [u8],
    signed: bool,
}

/// `case` with each signature applied to every commit, ref tip or new
/// commit, carrying that message, in its expected outcome and in its recorded
/// oracle outcome if it has one: a `fixed:` case's oracle signs as wsx does.
fn with_signatures(mut case: Case, signatures: &[Signature]) -> Case {
    let mut expected = case
        .expected
        .take()
        .expect("the registered case states an outcome");
    sign(&mut expected, signatures);
    if let OracleSide::Recorded(record) = &mut case.oracle {
        sign(record, signatures);
    }
    case.expect(expected)
}

/// `outcome` with each signature applied to every commit carrying that
/// message. A message no commit carries panics, so a stale message cannot
/// leave a signature unasserted.
fn sign(outcome: &mut Outcome, signatures: &[Signature]) {
    for Signature { message, signed } in signatures {
        let tips: Vec<&mut CommitExpect> = match &mut outcome.refs {
            RefsExpect::Exactly { refs, .. } => refs
                .iter_mut()
                .filter_map(|entry| match &mut entry.tip {
                    Tip::Commit(commit) => Some(commit),
                    Tip::Unchanged => None,
                })
                .collect(),
            RefsExpect::Unchanged => Vec::new(),
        };
        let mut found = 0;
        for commit in tips.into_iter().chain(outcome.new_commits.iter_mut()) {
            if commit.message == *message {
                commit.signed = *signed;
                found += 1;
            }
        }
        assert!(
            found > 0,
            "no expected commit has the message \"{}\"",
            message.escape_ascii()
        );
    }
}

/// `sync::self_test`'s subject with signing declared: the merge the sync
/// creates is signed, while `feat: add b`, made in a scratch clone with no
/// signing config and only fetched by the sync, is not.
fn sync_merge_signed() -> Scenario {
    let mut subject = registered("sync", "self_test").subject;
    subject.config = subject.config.with_signing();
    Scenario::new(
        "sync_merge_signed",
        with_signatures(
            subject,
            &[
                Signature {
                    message: SYNC_MERGE,
                    signed: true,
                },
                Signature {
                    message: FETCHED,
                    signed: false,
                },
            ],
        ),
    )
}

/// `cut::self_test`'s subject with signing declared: the replayed commit is
/// signed. Its counterpart is `cut::self_test`'s subject unchanged, whose
/// config declares no signing: the same replay is unsigned. Both keep
/// `cut::self_test`'s class: `fixed:` for the PR branch's upstream alone.
fn cut_replay_signed() -> Scenario {
    let mut subject = registered("cut", "self_test").subject;
    subject.config = subject.config.with_signing();
    let counterpart = registered("cut", "self_test").subject;
    Scenario::new(
        "cut_replay_signed",
        with_signatures(
            subject,
            &[Signature {
                message: REPLAYED,
                signed: true,
            }],
        ),
    )
    .control(
        Role::Counterpart,
        with_signatures(
            counterpart,
            &[Signature {
                message: REPLAYED,
                signed: false,
            }],
        ),
    )
}

scenarios! {
    sync_merge_signed: oracle, rust = "Rust side not enabled until phase 7";
    cut_replay_signed: oracle, rust = "Rust side not enabled until phase 9";
}
