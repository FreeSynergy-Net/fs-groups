//! Parent requests: a group asks to nest under a parent, the parent's side approves or declines.
//!
//! Pattern: Repository + State — **provisional**: who consents to a link is still the owner's to
//! answer. `request_parent` only writes a request; the link `fs:parents` exists once
//! `approve_parent` ran, and the mirror only ever sees approved links. The cycle rule is checked when
//! asking and again when approving, under the write lock.
//!
//! What is and is not enforced: the approver is the repository's `actor` — whatever string the
//! caller built the `GroupRepo` with. Nothing authenticates it, here or in `fs-groups-server` (the
//! transport knows no caller yet, `DESIGN.md`), so this is a check of a name, not of a person. It may approve if it holds `manage` on the parent by `may_do`, or, while the parent has no
//! delegation at all, if it is the one who created the parent. `grant` itself is not guarded yet,
//! so anybody who can call it can give themselves `manage`.

use fs_objects::{ObjectId, Order, Query, StoredObject};
use serde_json::json;

use super::codec;
use super::consts::{AS2_TYPE, CONTEXT, PARENT_REQUEST_TYPE};
use super::envelope::context;
use super::handle::{GroupRepo, Writer};
use crate::error::{Error, Result};
use crate::graph::MAX_NESTING_DEPTH;
use crate::model::{Group, NestingMode, ParentRequest, RequestKind, RequestState, Right};

impl GroupRepo {
    /// `group.nest`: ask for `parent` to become a parent of `child`, with the mode the parent
    /// preselects for new children (`default_mode`, [`NestingMode::BottomUp`] unless changed).
    /// See [`GroupRepo::request_parent_in`].
    pub async fn request_parent(&self, child: ObjectId, parent: ObjectId) -> Result<ParentRequest> {
        let mode = self.require(parent).await?.default_mode;
        self.request_parent_in(child, parent, mode).await
    }

    /// `group.nest`: ask for `parent` to become a parent of `child` with an edge of `mode`. Nothing is
    /// linked yet. Refused when it would close a loop, or when the link exists (in any mode: change
    /// it with `request_mode_change`). Asking again while a request is pending returns that request
    /// (a pending request with another mode is a conflict).
    pub async fn request_parent_in(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<ParentRequest> {
        let w = self.writer().await;
        let g = w.require(child).await?;
        w.refuse_cycle(child, parent).await?;
        if g.parents.contains(&parent) {
            return Err(Error::Conflict(format!(
                "{parent} is already a parent of {child}"
            )));
        }
        w.new_request(child, parent, RequestKind::Link, mode).await
    }

    /// `group.nest`: approve on behalf of the parent. The link is made (the loop rule is checked
    /// again). Refused with [`Error::Forbidden`] unless the actor may decide for the parent (see the
    /// module notes). Returns the child with its new parent.
    pub async fn approve_parent(&self, request: ObjectId) -> Result<Group> {
        let w = self.writer().await;
        let (o, r) = w.pending_request(request).await?;
        w.may_decide_for(r.parent).await?;
        let mut child = w.require(r.child).await?;
        match r.kind {
            RequestKind::Link => {
                w.refuse_cycle(r.child, r.parent).await?;
                if !child.parents.contains(&r.parent) {
                    child.parents.push(r.parent);
                }
            }
            RequestKind::ChangeMode if !child.parents.contains(&r.parent) => {
                return Err(Error::Conflict(format!(
                    "{} is no longer a parent of {}",
                    r.parent, r.child
                )));
            }
            RequestKind::ChangeMode => {}
        }
        child.parent_modes.insert(r.parent, r.mode);
        child = w.save(&child).await?;
        w.set_request_state(o, RequestState::Approved).await?;
        Ok(child)
    }

    /// `group.nest`: decline on behalf of the parent. Same rule as [`GroupRepo::approve_parent`].
    pub async fn decline_parent(&self, request: ObjectId) -> Result<()> {
        let w = self.writer().await;
        let (o, r) = w.pending_request(request).await?;
        w.may_decide_for(r.parent).await?;
        w.set_request_state(o, RequestState::Declined).await
    }

    /// `group.nest`: the requests made to `parent`, in any state, oldest first.
    pub async fn parent_requests(&self, parent: ObjectId) -> Result<Vec<ParentRequest>> {
        self.require(parent).await?;
        self.requests_where(&[("parent_of", parent)]).await
    }

    async fn requests_where(&self, keys: &[(&str, ObjectId)]) -> Result<Vec<ParentRequest>> {
        let mut q = Query::new()
            .of_type(PARENT_REQUEST_TYPE)
            .in_context(CONTEXT)
            .ordered(Order::PublishedAsc);
        for (k, v) in keys {
            q = q.with_index(*k, v.to_string());
        }
        Ok(self
            .reads
            .query(&q)
            .await?
            .iter()
            .filter_map(codec::parent_request)
            .collect())
    }
}

impl Writer<'_> {
    /// Store a pending request, or return the pending one of the same kind for the same pair.
    pub(super) async fn new_request(
        &self,
        child: ObjectId,
        parent: ObjectId,
        kind: RequestKind,
        mode: NestingMode,
    ) -> Result<ParentRequest> {
        let pending = self
            .requests_where(&[("child", child), ("parent_of", parent)])
            .await?
            .into_iter()
            .find(|r| r.state == RequestState::Pending && r.kind == kind);
        if let Some(same) = pending {
            if same.mode == mode {
                return Ok(same);
            }
            return Err(Error::Conflict(format!(
                "request {} is pending with mode {}",
                same.id,
                same.mode.as_str()
            )));
        }
        let env = self.fresh_envelope(
            PARENT_REQUEST_TYPE,
            &format!("{child} asks {parent} to be its parent"),
        );
        let content = json!({
            "@context": context(), "type": PARENT_REQUEST_TYPE, "fs:child": child.to_string(),
            "fs:parent": parent.to_string(), "fs:state": RequestState::Pending.as_str(),
            "fs:kind": kind.as_str(), "fs:mode": mode.as_str(),
        });
        let obj = StoredObject::new(env, AS2_TYPE, content)
            .with_index("child", child.to_string())
            .with_index("parent_of", parent.to_string())
            .with_index("state", RequestState::Pending.as_str())
            .with_index("kind", kind.as_str())
            .with_index("mode", mode.as_str());
        let id = obj.envelope.id;
        self.put(obj).await?;
        Ok(ParentRequest {
            id,
            child,
            parent,
            requester: self.actor.clone(),
            state: RequestState::Pending,
            kind,
            mode,
        })
    }

    async fn pending_request(&self, id: ObjectId) -> Result<(StoredObject, ParentRequest)> {
        let (o, r) = self
            .load(id)
            .await?
            .and_then(|o| codec::parent_request(&o).map(|r| (o, r)))
            .ok_or(Error::NotFound {
                what: "parent request",
                id,
            })?;
        if r.state == RequestState::Pending {
            Ok((o, r))
        } else {
            Err(Error::Conflict(format!(
                "request {id} is already {}",
                r.state.as_str()
            )))
        }
    }

    async fn set_request_state(&self, mut o: StoredObject, state: RequestState) -> Result<()> {
        o.index.insert("state".into(), vec![state.as_str().into()]);
        o.content["fs:state"] = json!(state.as_str());
        self.put(o).await
    }

    /// Both groups exist, `parent` is not `child` or one of its descendants, and the chain stays short.
    async fn refuse_cycle(&self, child: ObjectId, parent: ObjectId) -> Result<()> {
        let graph = self.known_graph(child).await?;
        if !graph.contains(parent) {
            return Err(Error::NotFound {
                what: "group",
                id: parent,
            });
        }
        if let Some(path) = graph.path_up(parent, child) {
            return Err(Error::Cycle {
                group: child,
                parent,
                path,
            });
        }
        // a chain through the new edge: what is above the parent, the edge, what is below the child
        let depth = graph.longest_chain(parent, true) + 1 + graph.longest_chain(child, false);
        if depth > MAX_NESTING_DEPTH {
            return Err(Error::TooDeep {
                group: child,
                parent,
                depth,
            });
        }
        Ok(())
    }

    /// Whether the actor may answer for `parent`: nobody holds any delegation on `parent` yet and the
    /// actor created it, or it holds `manage` there (`may_do`, which loads the whole graph, so the
    /// cheap case is looked at first).
    async fn may_decide_for(&self, parent: ObjectId) -> Result<()> {
        let no_delegation = self.delegations(parent).await?.is_empty();
        let created_by_actor = self
            .load(parent)
            .await?
            .is_some_and(|o| o.envelope.attributed_to.as_deref() == Some(self.actor.as_str()));
        // an agent never answers for a group (ADR 0056), as creator or otherwise
        let human = self.is_known_human(&self.actor).await?;
        if human
            && ((no_delegation && created_by_actor)
                || self.may_do(parent, &self.actor, Right::Manage).await?)
        {
            Ok(())
        } else {
            Err(Error::Forbidden(format!(
                "{} may not answer for group {parent}",
                self.actor
            )))
        }
    }
}
