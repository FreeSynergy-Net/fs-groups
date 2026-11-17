//! Where an effective member comes from.
//!
//! Pattern: Value object — the answer of "who belongs to this group, and why".

use fs_objects::ObjectId;

/// Why a person belongs to a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A direct membership of the asked group.
    Direct,
    /// Inherited from this group (a direct member there), along the path in `via`.
    Inherited(ObjectId),
}

/// A person who belongs to a group, with the origin and the paths (modes of the edges respected).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveOrigin {
    /// The person.
    pub person: String,
    /// Direct, or inherited from which group.
    pub origin: Origin,
    /// Shortest path from the asked group to the group of the direct membership (just the asked group
    /// for a direct member); each step is one edge a member flows along (against the flow).
    pub via: Vec<ObjectId>,
    /// Other groups the person is a direct member of and reaches the asked group through, each with
    /// its shortest path, nearest first. Empty when the person has one source only.
    pub also: Vec<Vec<ObjectId>>,
}
