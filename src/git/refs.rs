//! Peeling object ids to the commits git names for them.

use super::repo::Repo;
use super::{Error, ObjectId};

impl Repo {
    /// The commit `id` names, peeling annotated tags as git's
    /// `<id>^{commit}` does. No ref is read and no revspec is parsed: `id`
    /// is decoded as hex, so a name or a short prefix is refused.
    pub fn peel_commit(&self, id: &ObjectId) -> Result<ObjectId, Error> {
        self.peeled_commit(id)
            .map(|commit| ObjectId::from_gix(&commit.id))
    }

    /// The commit `id` peels to, as gix's commit. A decode failure is
    /// `ObjectId::to_gix`'s error; a missing object, or one that does not
    /// peel to a commit, is `Error::Gix` naming `id`.
    fn peeled_commit(&self, id: &ObjectId) -> Result<gix::Commit<'_>, Error> {
        let gix_id = id.to_gix()?;
        self.repo
            .find_object(gix_id)
            .map_err(|err| err.to_string())
            .and_then(|object| object.peel_to_commit().map_err(|err| err.to_string()))
            .map_err(|message| Error::Gix {
                what: format!("peel {id} to a commit"),
                message,
            })
    }
}
