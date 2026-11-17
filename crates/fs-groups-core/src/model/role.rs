//! The fixed role list of a group (ADR 0086: typed, not an opaque string).
//!
//! Pattern: Value object (sum type) — `Member` < `Manager` < `Owner`, plus `Legacy` for a role text
//! stored before the list was fixed. What a role *allows* is decided in one place, `crate::protect`.

/// The role of a person in a group.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Role {
    /// A plain member. Stored as the empty text (what the wire calls "plain member").
    #[default]
    Member,
    /// Manager ("Verwalter"): may administer the group. Counts as admin (an agent cannot hold it).
    Manager,
    /// Owner ("Eigentümer"): the manager who may also delete the group and hand the ownership over.
    Owner,
    /// A role text from before the list was fixed (or from outside). It is kept and shown, counts for
    /// no protection and no admin right of its own, and cannot be written through the typed API.
    /// Use [`Role::Member`], [`Role::Manager`] or [`Role::Owner`] instead.
    Legacy(String),
}

impl Role {
    /// Read a stored or wire role text. Case and padding do not matter. `""`, `member` and `mitglied`
    /// are [`Role::Member`]; `manager`, `admin` and `verwalter` are [`Role::Manager`]; `owner`,
    /// `eigentümer` and `eigentuemer` are [`Role::Owner`]; anything else is [`Role::Legacy`].
    pub fn parse(text: &str) -> Role {
        match crate::text::fold(text).as_str() {
            "" | "member" | "mitglied" => Role::Member,
            "manager" | "admin" | "verwalter" => Role::Manager,
            "owner" | "eigentümer" | "eigentuemer" => Role::Owner,
            _ => Role::Legacy(text.trim().to_string()),
        }
    }

    /// The text that is stored and sent: `""`, `manager`, `owner`, or the legacy text unchanged.
    pub fn as_str(&self) -> &str {
        match self {
            Role::Member => "",
            Role::Manager => "manager",
            Role::Owner => "owner",
            Role::Legacy(s) => s,
        }
    }

    /// Whether the role may administer the group: manager or owner.
    pub fn is_admin(&self) -> bool {
        matches!(self, Role::Manager | Role::Owner)
    }

    /// Whether this is a role the typed API may write (everything but [`Role::Legacy`]).
    pub fn is_known(&self) -> bool {
        !matches!(self, Role::Legacy(_))
    }
}
