//! Edges, the member graph and its checks (cycle in the tree and in the member graph, depth).
//!
//! Pattern: Specification (pure functions over an edge list; ids are any ordered copy type).

use std::collections::{BTreeMap, VecDeque};

/// The mode of an edge (the same three as `NestingMode`, kept apart so the module can be swapped).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    BottomUp,
    TopDown,
    Single,
}

/// An edge "child belongs to parent" with its mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Edge<I> {
    pub child: I,
    pub parent: I,
    pub mode: Mode,
}

impl<I: Copy> Edge<I> {
    /// The arc of the member graph `(who becomes member, of what)`; `Single` has none.
    pub(crate) fn arc(&self) -> Option<(I, I)> {
        match self.mode {
            Mode::BottomUp => Some((self.child, self.parent)),
            Mode::TopDown => Some((self.parent, self.child)),
            Mode::Single => None,
        }
    }
}

/// All arcs of the member graph.
pub(crate) fn arcs<I: Copy>(edges: &[Edge<I>]) -> Vec<(I, I)> {
    edges.iter().filter_map(Edge::arc).collect()
}

/// A way `from` to `to` over `pairs` (breadth first), when there is one.
fn path<I: Copy + Ord>(pairs: &[(I, I)], from: I, to: I) -> Option<Vec<I>> {
    let mut back: BTreeMap<I, I> = BTreeMap::new();
    let mut queue = VecDeque::from([from]);
    while let Some(at) = queue.pop_front() {
        if at == to {
            let mut way = vec![at];
            let mut cur = at;
            while let Some(prev) = back.get(&cur) {
                way.push(*prev);
                cur = *prev;
            }
            way.reverse();
            return Some(way);
        }
        for (a, b) in pairs {
            if *a == at && *b != from && !back.contains_key(b) {
                back.insert(*b, at);
                queue.push_back(*b);
            }
        }
    }
    None
}

/// The longest chain (number of pairs) in an acyclic graph.
pub(crate) fn longest<I: Copy + Ord>(pairs: &[(I, I)]) -> u32 {
    fn from<I: Copy + Ord>(at: I, pairs: &[(I, I)], memo: &mut BTreeMap<I, u32>) -> u32 {
        if let Some(d) = memo.get(&at) {
            return *d;
        }
        let d = pairs
            .iter()
            .filter(|(a, _)| *a == at)
            .map(|(_, b)| 1 + from(*b, pairs, memo))
            .max()
            .unwrap_or(0);
        memo.insert(at, d);
        d
    }
    let mut memo = BTreeMap::new();
    pairs
        .iter()
        .map(|(a, _)| from(*a, pairs, &mut memo))
        .max()
        .unwrap_or(0)
}

/// The cycle `new` would close among `edges`: in the tree (all modes) or in the member graph (after turning the
/// `TopDown` edges round). The path starts and ends with the same group.
pub(crate) fn find_cycle<I: Copy + Ord>(edges: &[Edge<I>], new: &Edge<I>) -> Option<Vec<I>> {
    let tree: Vec<(I, I)> = edges.iter().map(|e| (e.child, e.parent)).collect();
    if let Some(mut way) = path(&tree, new.parent, new.child) {
        way.insert(0, new.child);
        return Some(way);
    }
    let (u, v) = new.arc()?;
    let mut way = path(&arcs(edges), v, u)?;
    way.insert(0, u);
    Some(way)
}

/// Whether the tree or the member graph, with `new` added, is deeper than `max` edges.
pub(crate) fn too_deep<I: Copy + Ord>(edges: &[Edge<I>], new: &Edge<I>, max: u32) -> bool {
    let mut all: Vec<Edge<I>> = edges.to_vec();
    all.push(*new);
    let tree: Vec<(I, I)> = all.iter().map(|e| (e.child, e.parent)).collect();
    longest(&tree) > max || longest(&arcs(&all)) > max
}

/// All groups reachable from `start` over the tree (`up` = towards the parents), breadth first, each once; a group
/// for which `pass` says no is neither listed nor walked through.
pub(crate) fn reach<I: Copy + Ord>(
    edges: &[Edge<I>],
    start: I,
    up: bool,
    pass: &dyn Fn(I) -> bool,
) -> Vec<I> {
    let mut seen: Vec<I> = Vec::new();
    let mut queue = VecDeque::from([start]);
    while let Some(at) = queue.pop_front() {
        for e in edges {
            let (from, to) = if up {
                (e.child, e.parent)
            } else {
                (e.parent, e.child)
            };
            if from == at && to != start && !seen.contains(&to) && pass(to) {
                seen.push(to);
                queue.push_back(to);
            }
        }
    }
    seen
}
