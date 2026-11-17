//! Effective membership: through nested groups, with origin and path, the modes of the edges respected.
//!
//! Pattern: Visitor over the in-memory `MemberFlow` — members flow along the arcs the modes make
//! (`BottomUp` up, `TopDown` down, `Single` not at all). The walk visits each group once, so it ends
//! on any data. The shortest path wins; other sources are listed as alternatives.

use std::collections::BTreeSet;

use fs_objects::{ObjectId, Query};

use super::handle::GroupRepo;
use crate::error::Result;
use crate::flow::MemberFlow;
use crate::model::{EffectiveMember, EffectiveOrigin, Member, Origin};

impl GroupRepo {
    /// `member.read`: everyone who belongs to `group`, each person once, with where it comes from:
    /// a direct membership, or inherited from the group of a direct membership along `via` (the
    /// shortest path), the modes of all edges respected. `also` lists further sources, each with its
    /// shortest path.
    pub async fn effective_origins(&self, group: ObjectId) -> Result<Vec<EffectiveOrigin>> {
        self.require(group).await?;
        let flow = MemberFlow::from_groups(&self.all_groups().await?);
        let mut out: Vec<EffectiveOrigin> = vec![];
        for (node, path) in flow.sources_of(group) {
            for m in self.direct(node).await? {
                let Member::Person(p) = m.member else {
                    continue;
                };
                match out.iter_mut().find(|e| e.person == p) {
                    Some(e) => {
                        if !e.also.contains(&path) && e.via != path {
                            e.also.push(path.clone());
                        }
                    }
                    None => out.push(EffectiveOrigin {
                        person: p,
                        origin: if node == group {
                            Origin::Direct
                        } else {
                            Origin::Inherited(node)
                        },
                        via: path.clone(),
                        also: vec![],
                    }),
                }
            }
        }
        Ok(out)
    }

    /// `member.read`: everyone who belongs to `group`, each person once, with the shortest path:
    /// `[group]` for a direct member, further along the way members flow otherwise.
    pub async fn effective(&self, group: ObjectId) -> Result<Vec<EffectiveMember>> {
        Ok(self
            .effective_origins(group)
            .await?
            .into_iter()
            .map(|e| EffectiveMember {
                person: e.person,
                via: e.via,
            })
            .collect())
    }

    /// `member.read`: the groups a person belongs to: the ones they are in, and every group their
    /// membership flows into (modes respected).
    pub async fn groups_of(&self, person: &str) -> Result<Vec<ObjectId>> {
        let person = person.trim();
        let groups = self.all_groups().await?;
        let known: BTreeSet<_> = groups.iter().map(|g| g.id).collect();
        let flow = MemberFlow::from_groups(&groups);
        let q = Query::new()
            .with_index("member_kind", "person")
            .with_index("member", person);
        let mut all = BTreeSet::new();
        for m in self
            .memberships(q)
            .await?
            .into_iter()
            .filter(|m| known.contains(&m.group))
        {
            all.extend(flow.reach_from(m.group));
        }
        Ok(all.into_iter().collect())
    }

    /// `member.read`: whether `person` belongs to `group`, through nesting too.
    pub async fn is_member(&self, group: ObjectId, person: &str) -> Result<bool> {
        self.require(group).await?;
        Ok(self.groups_of(person).await?.contains(&group))
    }
}
