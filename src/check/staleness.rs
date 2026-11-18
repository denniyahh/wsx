//! How stale the local integration ref is (DIV-04): the two times of D-12,
//! as amended, which the check reports and never judges (D-13).
//!
//! `ref_moved_at` is the time of the newest reflog entry of the integration
//! ref's remote-tracking ref: when this repository last moved that ref, not
//! when it last asked the remote. A fetch that finds nothing new adds no
//! entry, and a fresh clone has no reflog for it. `fetched_at` is the newest
//! `FETCH_HEAD` modification time across every worktree: when any worktree
//! last fetched anything, from any remote, not that the integration ref is
//! current. A local integration branch (no base remote) is not fetched, so
//! neither time applies to it and neither is read.
//!
//! Neither time is ever a failure: there is no threshold, nothing here sets
//! an exit code, and a read that fails is a null time with its reason. Times
//! are whole unix seconds; an age is the run's one clock reading minus the
//! time, negative when the time is after it (clock skew). The times are read
//! once per run with no lock, so a fetch running at the same time can make
//! either one older or newer by that fetch, never an error.

use std::fmt::Display;

use crate::git::{Git, Repo, c_quote};

/// The prefix of a remote-tracking ref's full name.
const REMOTE_TRACKING: &[u8] = b"refs/remotes/";

/// One staleness time as the check read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeRead {
    /// Not read: staleness does not apply to the integration ref.
    NotRead,
    /// The time, in whole unix seconds.
    At(i64),
    /// No time: why, when it is absent or its read failed.
    Unknown(String),
}

/// Where a staleness time comes from.
#[derive(Clone, Copy, Debug)]
pub enum Source<'a> {
    /// The reflog of the ref with this full name.
    Reflog(&'a [u8]),
    /// The newest `FETCH_HEAD` of any worktree.
    FetchHead,
}

/// How stale the integration ref is: both times and the clock reading their
/// ages are measured against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staleness {
    /// Whether the integration ref is a remote-tracking ref, the only kind a
    /// fetch moves.
    pub applies: bool,
    /// The newest reflog entry of the integration ref.
    pub ref_moved_at: TimeRead,
    /// The newest `FETCH_HEAD` modification time.
    pub fetched_at: TimeRead,
    /// The run's clock reading, in whole unix seconds.
    pub now: i64,
}

impl Staleness {
    /// Neither time read yet; whether staleness applies to `full_ref`.
    pub fn unread(full_ref: &[u8], now: i64) -> Self {
        Staleness {
            applies: applies(full_ref),
            ref_moved_at: TimeRead::NotRead,
            fetched_at: TimeRead::NotRead,
            now,
        }
    }
}

/// Whether staleness applies to the integration ref `full_ref`: exactly
/// when it is a remote-tracking ref.
pub fn applies(full_ref: &[u8]) -> bool {
    full_ref.starts_with(REMOTE_TRACKING)
}

/// How stale the integration ref `full_ref` is in `repo`, against the clock
/// reading `now`. Only a remote-tracking ref is read: one reflog child and
/// one stat of every worktree's `FETCH_HEAD`, neither contacting a remote.
/// Each answer, a failed read included, goes through `time_of`, so nothing
/// read here can fail the check (D-13).
pub fn read(repo: &Repo, git: &Git, full_ref: &[u8], now: i64) -> Staleness {
    let unread = Staleness::unread(full_ref, now);
    if !unread.applies {
        return unread;
    }
    Staleness {
        ref_moved_at: time_of(Source::Reflog(full_ref), repo.ref_moved_at(git, full_ref)),
        fetched_at: time_of(Source::FetchHead, repo.fetched_at()),
        ..unread
    }
}

/// The time one staleness read answered, from `source`: a time, or why
/// there is none. An error becomes `Unknown` with its text, so no read error
/// reaches the check's result (D-13).
pub fn time_of<E: Display>(source: Source<'_>, answer: Result<Option<i64>, E>) -> TimeRead {
    match (answer, source) {
        (Ok(Some(time)), _) => TimeRead::At(time),
        (Ok(None), Source::Reflog(full_ref)) => {
            TimeRead::Unknown(format!("no reflog for {}", quoted(full_ref)))
        }
        (Ok(None), Source::FetchHead) => TimeRead::Unknown("no FETCH_HEAD".to_owned()),
        (Err(err), Source::Reflog(full_ref)) => TimeRead::Unknown(format!(
            "the reflog of {} could not be read: {err}",
            quoted(full_ref)
        )),
        (Err(err), Source::FetchHead) => {
            TimeRead::Unknown(format!("FETCH_HEAD could not be read: {err}"))
        }
    }
}

/// Seconds in a minute, an hour and a day.
const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

/// `seconds` as the human line writes an age, in its two largest units; a
/// negative age (a time after now) is `0s`.
pub fn age_text(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds >= DAY {
        format!("{}d {}h", seconds / DAY, seconds % DAY / HOUR)
    } else if seconds >= HOUR {
        format!("{}h {}m", seconds / HOUR, seconds % HOUR / MINUTE)
    } else if seconds >= MINUTE {
        format!("{}m {}s", seconds / MINUTE, seconds % MINUTE)
    } else {
        format!("{seconds}s")
    }
}

/// `bytes` in git's C-style quoting, for prose.
fn quoted(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&c_quote(bytes)).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The integration ref of the fixtures.
    const ORIGIN_DEVELOP: &[u8] = b"refs/remotes/origin/develop";

    /// Each unit boundary, and a negative age.
    #[test]
    fn age_text_at_each_boundary() {
        let cases: [(i64, &str); 8] = [
            (0, "0s"),
            (59, "59s"),
            (60, "1m 0s"),
            (3599, "59m 59s"),
            (3600, "1h 0m"),
            (86399, "23h 59m"),
            (86400, "1d 0h"),
            (-5, "0s"),
        ];
        let got: Vec<(i64, String)> = cases
            .iter()
            .map(|(seconds, _)| (*seconds, age_text(*seconds)))
            .collect();
        let want: Vec<(i64, String)> = cases
            .iter()
            .map(|(seconds, text)| (*seconds, (*text).to_owned()))
            .collect();
        assert_eq!(got, want);
    }

    /// A remote-tracking ref is fetched; a local branch is not.
    #[test]
    fn only_a_remote_tracking_ref_is_fetched() {
        assert!(applies(ORIGIN_DEVELOP), "refs/remotes/origin/develop");
        assert!(!applies(b"refs/heads/develop"), "refs/heads/develop");
    }

    /// A failed read is a null time with its reason, for either time; a time
    /// and an absent time are the controls.
    #[test]
    fn a_failed_read_is_a_null_time_with_its_reason() {
        let failed: Result<Option<i64>, &str> = Err("exit status 128");
        assert_eq!(
            time_of(Source::Reflog(ORIGIN_DEVELOP), failed),
            TimeRead::Unknown(
                "the reflog of refs/remotes/origin/develop could not be read: exit status 128"
                    .to_owned()
            )
        );
        assert_eq!(
            time_of(Source::FetchHead, failed),
            TimeRead::Unknown("FETCH_HEAD could not be read: exit status 128".to_owned())
        );
        assert_eq!(
            time_of(Source::Reflog(ORIGIN_DEVELOP), Ok::<_, &str>(Some(5))),
            TimeRead::At(5)
        );
        assert_eq!(
            time_of(Source::Reflog(ORIGIN_DEVELOP), Ok::<_, &str>(None)),
            TimeRead::Unknown("no reflog for refs/remotes/origin/develop".to_owned())
        );
        assert_eq!(
            time_of(Source::FetchHead, Ok::<_, &str>(None)),
            TimeRead::Unknown("no FETCH_HEAD".to_owned())
        );
    }
}
