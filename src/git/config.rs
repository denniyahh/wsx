//! Reads of the repository's own config: wsx's keys and the remotes it
//! defines. Each read takes the config gix froze when `Repo::discover` opened
//! the repository, so a value set later is seen only by a new `Repo`.

use gix::bstr::ByteSlice;

use super::repo::Repo;

impl Repo {
    /// Every value of `key` in this repository's own config file (gix's
    /// `Source::Local`, the common dir's `config`), in file order, as raw
    /// bytes. A global, system or `-c` value is never returned, so a value
    /// set for the machine cannot configure this repository (D-01).
    pub fn local_config_values(&self, key: &str) -> Vec<Vec<u8>> {
        self.repo
            .config_snapshot()
            .plumbing()
            .strings_filter(key, |meta| meta.source == gix::config::Source::Local)
            .unwrap_or_default()
            .into_iter()
            .map(Vec::from)
            .collect()
    }

    /// Whether gix lists the remote `name` among this repository's remotes.
    pub fn remote_configured(&self, name: &[u8]) -> bool {
        self.repo
            .remote_names()
            .iter()
            .any(|remote| remote.as_bytes() == name)
    }
}
