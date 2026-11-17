//! The group value and its AS2 `Group` form.
//!
//! Pattern: Value object + Adapter — `as2` / `from_content` map between the typed value and the
//! AS2 JSON that is the authoritative content (kind and parents are the `fs:` extension members).

use std::collections::BTreeMap;

use fs_objects::{Envelope, ObjectId};
use serde_json::{Value, json};

use super::{NestingMode, RightFlow};

/// Namespace of the members this program adds to AS2.
pub const NS: &str = "https://freesynergy.net/ns#";

/// A group: an AS2 `Group` with a free-text kind and any number of parents.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// Id (UUIDv7); the key for everything else.
    pub id: ObjectId,
    /// `https://<instance>/groups/<id>`.
    pub iri: String,
    /// Display name (also the envelope name, ADR 0032).
    pub name: String,
    /// Team, project, department, ... free text, not a fixed list.
    pub kind: String,
    /// Direct parents; a group may have several. Never contains a cycle.
    pub parents: Vec<ObjectId>,
    /// The mode of each edge to a parent (ADR 0082). A parent without an entry (an edge stored before
    /// modes existed) is [`NestingMode::BottomUp`]; read it with [`Group::mode_to`].
    pub parent_modes: BTreeMap<ObjectId, NestingMode>,
    /// The mode preselected for a new child of this group.
    pub default_mode: NestingMode,
    /// Whether and where this group passes its rights on; `None` when unset.
    pub right_flow: RightFlow,
    /// Its AS2 envelope, as stored (`Group.envelope` in `groups.capnp`).
    pub envelope: Envelope,
}

impl Group {
    /// The AS2 `Group` as JSON: the authoritative content of the stored object.
    pub fn as2(&self) -> Value {
        json!({
            "@context": ["https://www.w3.org/ns/activitystreams", {"fs": NS}],
            "type": "Group",
            "id": self.iri,
            "name": self.name,
            "fs:kind": self.kind,
            "fs:parents": self.parents.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "fs:rightFlow": self.right_flow.as_str(),
            "fs:parentModes": self.parent_modes.iter()
                .map(|(p, m)| (p.to_string(), json!(m.as_str()))).collect::<serde_json::Map<_, _>>(),
            "fs:defaultMode": self.default_mode.as_str(),
        })
    }

    /// The mode of the edge from this group to `parent` (`BottomUp` when none is recorded).
    pub fn mode_to(&self, parent: ObjectId) -> NestingMode {
        self.parent_modes.get(&parent).copied().unwrap_or_default()
    }

    /// The edge modes named in AS2 content: none recorded is fine (old data); an entry that is not
    /// `<group id>: <mode>` makes the group unreadable, like a corrupt parent.
    fn modes_in(content: &Value) -> Option<BTreeMap<ObjectId, NestingMode>> {
        let Some(map) = content.get("fs:parentModes") else {
            return Some(BTreeMap::new());
        };
        map.as_object()?
            .iter()
            .map(|(k, v)| Some((k.parse().ok()?, NestingMode::parse(v.as_str()?)?)))
            .collect()
    }

    /// The parents named in AS2 content: `None` if the member is missing or any entry is not a group id
    /// (a corrupt entry must not be dropped quietly: the group is then unreadable).
    pub fn parents_in(content: &Value) -> Option<Vec<ObjectId>> {
        content
            .get("fs:parents")?
            .as_array()?
            .iter()
            .map(|v| v.as_str()?.parse().ok())
            .collect()
    }

    /// Read a group back from its AS2 content. `None` if it is not a well-formed group.
    pub fn from_content(envelope: Envelope, content: &Value) -> Option<Group> {
        let text = |k: &str| content.get(k)?.as_str().map(str::to_string);
        let parents = Self::parents_in(content)?;
        let parent_modes = Self::modes_in(content)?;
        let default_mode = match content.get("fs:defaultMode") {
            None => NestingMode::default(),
            Some(v) => NestingMode::parse(v.as_str()?)?,
        };
        Some(Group {
            id: envelope.id,
            iri: text("id")?,
            name: text("name")?,
            // A group without a kind is still a group (the kind is free text and may be empty).
            kind: text("fs:kind").unwrap_or_default(),
            parents,
            parent_modes,
            default_mode,
            // An object written before the setting existed has none: it passes nothing on.
            right_flow: text("fs:rightFlow")
                .and_then(|s| RightFlow::parse(&s))
                .unwrap_or_default(),
            envelope,
        })
    }
}
