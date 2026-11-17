//! Modes of nesting edges: read, set the default of a parent, change an edge (ADR 0082).
//!
//! Pattern: Repository + State — a mode change that adds a flow of members (to `BottomUp` or
//! `TopDown`) needs the other side's consent, like a new edge: it is a `ParentRequest` of kind
//! `ChangeMode`, answered with `approve_parent` / `decline_parent`. A change to `Single` removes the
//! flow and is made at once. Which way members flow is decided in `crate::flow`.

use fs_objects::ObjectId;

use super::handle::GroupRepo;
use crate::error::{Error, Result};
use crate::model::{Group, NestingMode, ParentRequest, RequestKind};

impl GroupRepo {
    /// The mode of the edge `child -> parent`; [`Error::NotFound`] if `parent` is no parent of `child`.
    pub async fn parent_mode(&self, child: ObjectId, parent: ObjectId) -> Result<NestingMode> {
        let g = self.require(child).await?;
        if g.parents.contains(&parent) {
            Ok(g.mode_to(parent))
        } else {
            Err(Error::NotFound {
                what: "nesting edge",
                id: parent,
            })
        }
    }

    /// The mode a new child of `group` is preselected with. Only new requests are affected.
    pub async fn set_default_mode(&self, group: ObjectId, mode: NestingMode) -> Result<Group> {
        let w = self.writer().await;
        let mut g = w.require(group).await?;
        g.default_mode = mode;
        w.save(&g).await
    }

    /// Change an edge to `NestingMode::Single` at once: nothing flows afterwards, so the other side
    /// loses nothing. Any other target mode needs consent: [`Error::Forbidden`], use
    /// [`GroupRepo::request_mode_change`].
    pub async fn set_single(&self, child: ObjectId, parent: ObjectId) -> Result<Group> {
        let w = self.writer().await;
        let mut g = w.require(child).await?;
        Self::edge_for_change(&g, parent, NestingMode::Single)?;
        g.parent_modes.insert(parent, NestingMode::Single);
        w.save(&g).await
    }

    /// Ask to change the mode of the existing edge `child -> parent` to `BottomUp` or `TopDown`.
    /// Nothing changes until `approve_parent` ran on the request. `Single` and the same mode are
    /// refused ([`Error::Conflict`]; for `Single` use [`GroupRepo::set_single`]).
    pub async fn request_mode_change(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<ParentRequest> {
        let w = self.writer().await;
        let g = w.require(child).await?;
        let from = Self::edge_for_change(&g, parent, mode);
        match from {
            Err(Error::Forbidden(_)) => {}
            Err(e) => return Err(e),
            Ok(()) => {
                return Err(Error::Conflict(
                    "this change needs no consent: use set_single".into(),
                ));
            }
        }
        w.new_request(child, parent, RequestKind::ChangeMode, mode)
            .await
    }
}

impl GroupRepo {
    /// Checks shared by both ways of changing an edge. `Ok` means the change to `to` needs no consent;
    /// [`Error::Forbidden`] means it does; anything else is a refusal.
    fn edge_for_change(g: &Group, parent: ObjectId, to: NestingMode) -> Result<()> {
        if !g.parents.contains(&parent) {
            return Err(Error::NotFound {
                what: "nesting edge",
                id: parent,
            });
        }
        let from = g.mode_to(parent);
        if from == to {
            return Err(Error::Conflict(format!(
                "the edge is already {}",
                to.as_str()
            )));
        }
        if from.change_needs_consent(to) {
            Err(Error::Forbidden(format!(
                "changing {} to {} needs the parent's consent: ask with request_mode_change",
                from.as_str(),
                to.as_str()
            )))
        } else {
            Ok(())
        }
    }
}
