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

/// Base of the role IRIs: `…#role/member`, `…#role/manager`, `…#role/owner`, `…#role/legacy/<text>`.
const IRI_BASE: &str = "https://freesynergy.net/ns#role/";
const LEGACY: &str = "legacy/";

fn escape(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn unescape(text: &str) -> Option<String> {
    let mut out = vec![];
    let mut bytes = text.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hex = [bytes.next()?, bytes.next()?];
            out.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            out.push(b);
        }
    }
    String::from_utf8(out).ok()
}

impl Role {
    /// The role as an IRI, the form of `RoleRef` in the contract `groups_v1` (ADR 0052). The three
    /// known roles have fixed IRIs; a legacy text that is itself an absolute IRI stays as it is; any
    /// other legacy text is escaped under `…#role/legacy/`. [`Role::from_iri`] is the inverse.
    pub fn iri(&self) -> String {
        match self {
            Role::Member => format!("{IRI_BASE}member"),
            Role::Manager => format!("{IRI_BASE}manager"),
            Role::Owner => format!("{IRI_BASE}owner"),
            Role::Legacy(t) if is_iri(t) => t.clone(),
            Role::Legacy(t) => format!("{IRI_BASE}{LEGACY}{}", escape(t)),
        }
    }

    /// Inverse of [`Role::iri`]. An IRI that is none of the above is a [`Role::Legacy`] with the IRI
    /// as its text, so no role reference is lost. (A `Legacy` made by hand whose text is one of the
    /// three known role IRIs reads back as that known role.)
    pub fn from_iri(iri: &str) -> Role {
        let Some(rest) = iri.strip_prefix(IRI_BASE) else {
            return Role::Legacy(iri.to_string());
        };
        match rest {
            "member" => Role::Member,
            "manager" => Role::Manager,
            "owner" => Role::Owner,
            _ => rest
                .strip_prefix(LEGACY)
                .and_then(unescape)
                .map_or_else(|| Role::Legacy(iri.to_string()), Role::Legacy),
        }
    }

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

/// An absolute IRI: a scheme, `:`, and something after it.
fn is_iri(t: &str) -> bool {
    t.split_once(':').is_some_and(|(scheme, rest)| {
        !rest.is_empty()
            && scheme
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    })
}
