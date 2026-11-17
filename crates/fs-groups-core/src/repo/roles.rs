//! Roles of members: change a role, leave, hand the ownership over, delete as owner.
//!
//! Pattern: Repository + Specification — every change that could take the last owner or manager away
//! builds the "after" list and asks `crate::protect::check_change` (one place). Several writes happen
//! under the one write lock, in an order whose half-done state is safe (never without an owner).
//! Like the rest, the caller's identity is only a name that is compared (`DESIGN.md`).

use fs_objects::ObjectId;
use serde_json::json;

use super::codec;
use super::consts::MEMBERSHIP_TYPE;
use super::handle::{GroupRepo, Writer};
use crate::error::{Error, Result};
use crate::model::{Member, Membership, Role};
use crate::{protect, text};

impl GroupRepo {
    /// `member.add` with a role of the fixed list. A [`Role::Legacy`] text is refused.
    pub async fn add_role(
        &self,
        group: ObjectId,
        member: Member,
        role: &Role,
    ) -> Result<Membership> {
        known(role)?;
        self.add(group, member, role.as_str()).await
    }

    /// Change the role of a membership. Demoting the last owner (or, with no owner, the last manager)
    /// is refused. A new manager or owner must be known to be a human. If the person already holds
    /// the new role in this group, the two memberships become one.
    pub async fn set_role(&self, membership: ObjectId, role: &Role) -> Result<Membership> {
        known(role)?;
        let w = self.writer().await;
        let m = w.membership_of(membership).await?;
        let before = w.direct(m.group).await?;
        let after = with_role(&before, m.id, role);
        protect::check_change(m.group, &before, &after)?;
        w.require_role_holder(&m, role).await?;
        w.rewrite_role(&m, role).await?;
        w.membership_of(membership).await.or(Ok(Membership {
            role: role.as_str().into(),
            ..m
        }))
    }

    /// A person leaves a group (all their direct memberships end). The last owner (or, with no
    /// owner, the last manager) cannot leave. Not a direct member: [`Error::NotMember`].
    pub async fn leave(&self, group: ObjectId, person: &str) -> Result<()> {
        let person = text::person("person", person)?;
        let w = self.writer().await;
        let before = w.direct(group).await?;
        let mine: Vec<_> = before
            .iter()
            .filter(|m| m.member == Member::Person(person.clone()))
            .collect();
        if mine.is_empty() {
            return Err(Error::NotMember { group, person });
        }
        let after: Vec<_> = before
            .iter()
            .filter(|m| !mine.iter().any(|x| x.id == m.id))
            .cloned()
            .collect();
        protect::check_change(group, &before, &after)?;
        for m in mine {
            w.remove_object(m.id).await?;
        }
        Ok(())
    }

    /// Hand the ownership over, as one step: `to` (a direct member, known to be a human) becomes
    /// owner and `from` (an owner) becomes `keep`: [`Role::Manager`] or [`Role::Member`]. The new
    /// owner is written first, so a failure half way leaves two owners, never none.
    pub async fn transfer_ownership(
        &self,
        group: ObjectId,
        from: &str,
        to: &str,
        keep: &Role,
    ) -> Result<Membership> {
        if !matches!(keep, Role::Manager | Role::Member) {
            return Err(Error::Invalid(
                "the previous owner becomes manager or member".into(),
            ));
        }
        let (from, to) = (text::person("from", from)?, text::person("to", to)?);
        if from == to {
            return Err(Error::Invalid("the new owner is the current owner".into()));
        }
        let w = self.writer().await;
        let before = w.direct(group).await?;
        let find = |p: &str, only: Option<Role>| {
            before.iter().find(|m| {
                m.member == Member::Person(p.into())
                    && only.as_ref().is_none_or(|r| m.typed_role() == *r)
            })
        };
        let old = find(&from, Some(Role::Owner))
            .filter(|m| m.in_force)
            .ok_or_else(|| Error::Forbidden(format!("{from} is not an owner of group {group}")))?;
        let new = find(&to, None).ok_or(Error::NotMember {
            group,
            person: to.clone(),
        })?;
        let after = with_role(&with_role(&before, new.id, &Role::Owner), old.id, keep);
        protect::check_change(group, &before, &after)?;
        w.require_role_holder(new, &Role::Owner).await?;
        w.rewrite_role(new, &Role::Owner).await?;
        w.rewrite_role(old, keep).await?;
        w.membership_of(new.id).await
    }

    /// `group.delete`, asked by `actor`: only an owner may delete a group. A group without any owner
    /// (older data) may be deleted by a manager; the installer may always. Anyone else:
    /// [`Error::Forbidden`].
    pub async fn delete_by(&self, id: ObjectId, actor: &str) -> Result<()> {
        let actor = text::person("actor", actor)?;
        let members = self.direct(id).await?;
        let holds = |wanted: fn(&Role) -> bool| {
            members.iter().any(|m| {
                m.in_force && m.member == Member::Person(actor.clone()) && wanted(&m.typed_role())
            })
        };
        let any_owner = members
            .iter()
            .any(|m| m.in_force && m.typed_role() == Role::Owner);
        let allowed = self.installer() == Some(actor.as_str())
            || if any_owner {
                holds(|r| *r == Role::Owner)
            } else {
                holds(Role::is_admin)
            };
        if !allowed {
            return Err(Error::Forbidden(format!(
                "{actor} may not delete group {id}: only an owner may"
            )));
        }
        self.delete(id).await
    }
}

fn known(role: &Role) -> Result<()> {
    if role.is_known() {
        Ok(())
    } else {
        Err(Error::UnknownRole(role.as_str().to_string()))
    }
}

/// The list with one membership's role replaced.
fn with_role(list: &[Membership], id: ObjectId, role: &Role) -> Vec<Membership> {
    list.iter()
        .cloned()
        .map(|mut m| {
            if m.id == id {
                m.role = role.as_str().into();
            }
            m
        })
        .collect()
}

impl Writer<'_> {
    async fn membership_of(&self, id: ObjectId) -> Result<Membership> {
        self.load(id)
            .await?
            .filter(|o| o.envelope.object_type == MEMBERSHIP_TYPE)
            .and_then(|o| codec::membership(&o))
            .ok_or(Error::NotFound {
                what: "membership",
                id,
            })
    }

    /// A manager or owner must be known to be a human (ADR 0056).
    async fn require_role_holder(&self, m: &Membership, role: &Role) -> Result<()> {
        match (
            &m.member,
            role.is_admin() || self.is_admin_role(role.as_str()),
        ) {
            (Member::Person(p), true) => {
                self.require_human(p, &format!("role {:?} in {}", role.as_str(), m.group))
                    .await
            }
            _ => Ok(()),
        }
    }

    /// Write `role` into a membership. If the same person already holds that role in the group, this
    /// membership is dropped instead (one membership per person and role).
    async fn rewrite_role(&self, m: &Membership, role: &Role) -> Result<()> {
        if m.typed_role() == *role {
            return Ok(());
        }
        let twin = self
            .direct(m.group)
            .await?
            .into_iter()
            .any(|x| x.id != m.id && x.member == m.member && x.typed_role() == *role);
        if twin {
            return self.remove_object(m.id).await;
        }
        let Some(mut o) = self.load(m.id).await? else {
            return Err(Error::NotFound {
                what: "membership",
                id: m.id,
            });
        };
        o.content["org:role"] = json!(role.as_str());
        if role.as_str().is_empty() {
            o.index.remove("role");
        } else {
            o.index.insert("role".into(), vec![role.as_str().into()]);
        }
        self.put(o).await
    }
}
