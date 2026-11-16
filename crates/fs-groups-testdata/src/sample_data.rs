//! The fixed names of the sample: invented people (addresses on example.org only), groups and roles.
//!
//! Pattern: none (plain constants).

/// The humans of the sample (the first one asks and created every group).
pub(crate) const HUMANS: [&str; 10] = [
    "Alice Chair",
    "Bob Deputy",
    "Carol Lead",
    "Dave Outreach",
    "Erin Events",
    "Frank Stage",
    "Gina Guest",
    "Hank Union",
    "Ida Admin",
    "Jonas Intern",
];
/// The AI account of the sample.
pub(crate) const AGENT: &str = "Assistant Bot";
/// The person nobody knows to be human or AI.
pub(crate) const UNKNOWN: &str = "Visitor Unknown";

/// One group of the sample: name, kind, organisation, mirrored, sensitive, AI (a row of fixed data, so the flags stay plain).
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct GroupSpec {
    pub name: &'static str,
    pub kind: &'static str,
    pub organization: bool,
    pub mirror: bool,
    pub sensitive: bool,
    pub ai: bool,
}

const fn spec(name: &'static str, kind: &'static str, mirror: bool) -> GroupSpec {
    GroupSpec {
        name,
        kind,
        organization: false,
        mirror,
        sensitive: false,
        ai: false,
    }
}

/// The twelve groups (the last one goes to the trash).
pub(crate) fn groups() -> [GroupSpec; 12] {
    [
        GroupSpec {
            organization: true,
            ..spec("Association", "Verein", true)
        },
        spec("Outreach team", "Team", true),
        spec("Events team", "Team", true),
        spec("Stage crew", "Crew", true),
        spec("Chiefs", "Leitung", true),
        spec("Deputies", "Leitung", true),
        spec("Team leads", "Leitung", true),
        spec("Interns", "Gruppe", false),
        spec("Administrators", "Admin", true),
        GroupSpec {
            sensitive: true,
            ..spec("Union members", "Gruppe", false)
        },
        GroupSpec {
            ai: true,
            ..spec("Assistants", "KI-Gruppe", false)
        },
        spec("Archive", "Gruppe", false),
    ]
}

/// The edges: child, parent, mode (`b` bottom-up, `t` top-down, `s` single). Chiefs, Deputies and Team leads are
/// of unequal depth in the tree (1, 2 and 3 below the association's level) and still flow upwards by right.
pub(crate) const EDGES: [(&str, &str, char); 8] = [
    ("Outreach team", "Association", 'b'),
    ("Events team", "Association", 'b'),
    ("Stage crew", "Events team", 'b'),
    ("Chiefs", "Association", 's'),
    ("Deputies", "Chiefs", 't'),
    ("Team leads", "Deputies", 't'),
    ("Interns", "Association", 's'),
    ("Assistants", "Association", 's'),
];

/// Direct members: group, person, role label (`None` = plain member).
pub(crate) const MEMBERS: [(&str, &str, Option<&str>); 14] = [
    ("Chiefs", "Alice Chair", Some("Chair")),
    ("Deputies", "Bob Deputy", None),
    ("Team leads", "Carol Lead", None),
    ("Association", "Alice Chair", Some("Chair")),
    ("Outreach team", "Dave Outreach", None),
    ("Events team", "Erin Events", None),
    ("Stage crew", "Frank Stage", None),
    ("Administrators", "Alice Chair", Some("Administrator")),
    ("Administrators", "Ida Admin", None),
    ("Union members", "Hank Union", None),
    ("Union members", "Ida Admin", None),
    ("Assistants", "Assistant Bot", None),
    ("Interns", "Jonas Intern", None),
    ("Interns", "Visitor Unknown", None),
];
