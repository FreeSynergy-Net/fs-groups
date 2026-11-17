//! Where members flow along the nesting edges, each edge with its mode (ADR 0082).
//!
//! Pattern: Visitor over an in-memory directed graph — the "member graph": `BottomUp` makes an arc
//! child -> parent, `TopDown` an arc parent -> child, `Single` none. Pure and cycle-safe (a visited
//! set), no storage. The shortest path is the first one the breadth-first walk finds.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use fs_objects::ObjectId;

use crate::model::Group;

/// The member graph of all groups: who passes members on to whom.
#[derive(Clone, Debug, Default)]
pub struct MemberFlow {
    /// target -> the groups whose members flow into it.
    sources: BTreeMap<ObjectId, Vec<ObjectId>>,
    /// source -> the groups its members flow into.
    targets: BTreeMap<ObjectId, Vec<ObjectId>>,
}

impl MemberFlow {
    /// Build from groups. A parent that is not among them is ignored.
    pub fn from_groups(groups: &[Group]) -> Self {
        let known: BTreeSet<_> = groups.iter().map(|g| g.id).collect();
        let mut f = MemberFlow::default();
        for group in groups {
            for p in group.parents.iter().filter(|p| known.contains(p)) {
                if let Some((from, to)) = group.mode_to(*p).arc(group.id, *p) {
                    f.sources.entry(to).or_default().push(from);
                    f.targets.entry(from).or_default().push(to);
                }
            }
        }
        f
    }

    /// The groups whose direct members belong to `target`, with the shortest path from `target` to
    /// each (both ends included), nearest first; `target` itself comes first with `[target]`.
    pub fn sources_of(&self, target: ObjectId) -> Vec<(ObjectId, Vec<ObjectId>)> {
        let mut seen = BTreeSet::from([target]);
        let mut queue = VecDeque::from([(target, vec![target])]);
        let mut out = vec![(target, vec![target])];
        while let Some((cur, path)) = queue.pop_front() {
            for n in self.sources.get(&cur).into_iter().flatten() {
                if seen.insert(*n) {
                    let mut p = path.clone();
                    p.push(*n);
                    out.push((*n, p.clone()));
                    queue.push_back((*n, p));
                }
            }
        }
        out
    }

    /// Every group a direct member of `source` belongs to: `source` and all it flows into, once each.
    pub fn reach_from(&self, source: ObjectId) -> BTreeSet<ObjectId> {
        let mut seen = BTreeSet::from([source]);
        let mut queue = VecDeque::from([source]);
        while let Some(cur) = queue.pop_front() {
            for n in self.targets.get(&cur).into_iter().flatten() {
                if seen.insert(*n) {
                    queue.push_back(*n);
                }
            }
        }
        seen
    }
}
