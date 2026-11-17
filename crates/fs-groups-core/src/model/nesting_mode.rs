//! The mode of a nesting edge (ADR 0082, "Verschachtelung"): which way members flow.
//!
//! Pattern: Value object — the three modes and their stored text. The meaning (who becomes an
//! effective member of whom) is decided in one place, `crate::flow`.

/// How members flow along an edge from a child to its parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NestingMode {
    /// The child is a member of the parent (Kanidm style, Helfa): members count upward. The default,
    /// and the meaning of every edge stored before modes existed.
    #[default]
    BottomUp,
    /// The parent is a member of the child: members flow downward.
    TopDown,
    /// Order and display only: no member flows either way.
    Single,
}

impl NestingMode {
    /// Stable text used in the AS2 content (`fs:parentModes`, `fs:defaultMode`).
    pub fn as_str(self) -> &'static str {
        match self {
            NestingMode::BottomUp => "bottom_up",
            NestingMode::TopDown => "top_down",
            NestingMode::Single => "single",
        }
    }

    /// Inverse of [`NestingMode::as_str`].
    pub fn parse(s: &str) -> Option<NestingMode> {
        [Self::BottomUp, Self::TopDown, Self::Single]
            .into_iter()
            .find(|m| m.as_str() == s)
    }

    /// The direction members flow along the edge `child -> parent`: `Some((from, to))` means the
    /// members of `from` become effective members of `to`; `None` for [`NestingMode::Single`].
    pub fn arc<T: Copy>(self, child: T, parent: T) -> Option<(T, T)> {
        match self {
            NestingMode::BottomUp => Some((child, parent)),
            NestingMode::TopDown => Some((parent, child)),
            NestingMode::Single => None,
        }
    }

    /// Whether changing an edge from `self` to `to` needs the other side's consent. Going to
    /// [`NestingMode::Single`] takes nothing away from anybody's view of the other group, so it does
    /// not; every other change does (the contract of `groups_v1`).
    pub fn change_needs_consent(self, to: NestingMode) -> bool {
        self != to && to != NestingMode::Single
    }
}
