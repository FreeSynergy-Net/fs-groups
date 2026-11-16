//! The fixed sample: twelve groups in one tree with all three modes, built through the fake's own operations so it
//! stands for what a provider can reach (nothing is put in raw).
//!
//! Pattern: Builder (fixed data, applied step by step through the state operations; a step that fails is noted in
//! `problems`, which a test asserts empty).

use crate::fake::FakeGroups;
use crate::fake_mirror::FakeMirror;
use crate::groups_v1::shared::{ObjectId, PersonRef, Revision, Utc};
use crate::groups_v1::{
    DirectoryAccount, DirectoryGroup, GroupArt, GroupDraft, InvitationAnswer, NestingMode, Result,
    Right, RoleRef, Visibility,
};
use crate::ids::group_ref;
use crate::people::FakePeople;
use crate::sample_data::{AGENT, EDGES, HUMANS, MEMBERS, UNKNOWN, groups};
use crate::state::State;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The names of what was built: ids of groups, persons, roles, and the steps that failed.
#[derive(Default)]
struct Book {
    ids: BTreeMap<&'static str, ObjectId>,
    persons: BTreeMap<&'static str, PersonRef>,
    roles: BTreeMap<&'static str, RoleRef>,
    problems: Vec<String>,
}

impl Book {
    fn id(&self, name: &str) -> Option<ObjectId> {
        self.ids.get(name).copied()
    }
    fn person(&self, name: &str) -> Option<PersonRef> {
        self.persons.get(name).cloned()
    }
    fn role(&self, label: &str) -> Option<RoleRef> {
        self.roles.get(label).cloned()
    }
}

/// The sample: the fake, a directory, the consumed side, and the ids by name.
pub struct Sample {
    /// The provider, asked by Alice Chair.
    pub groups: FakeGroups,
    /// A Kanidm that holds an old state (a stale group, a foreign account on Administrators).
    pub mirror: FakeMirror,
    /// The consumed side.
    pub people: Arc<FakePeople>,
    book: Book,
}

impl Sample {
    /// The id of a group by its name.
    #[must_use]
    pub fn id(&self, name: &str) -> Option<ObjectId> {
        self.book.id(name)
    }

    /// A person by name (the humans, `Assistant Bot`, `Visitor Unknown`).
    #[must_use]
    pub fn person(&self, name: &str) -> Option<PersonRef> {
        self.book.person(name)
    }

    /// A role by label (`Chair`, `Treasurer`, `Administrator`).
    #[must_use]
    pub fn role(&self, label: &str) -> Option<RoleRef> {
        self.book.role(label)
    }

    /// Steps of the build that failed (empty when the sample is whole).
    #[must_use]
    pub fn problems(&self) -> &[String] {
        &self.book.problems
    }
}

fn mode_of(c: char) -> NestingMode {
    match c {
        'b' => NestingMode::BottomUp,
        't' => NestingMode::TopDown,
        _ => NestingMode::Single,
    }
}

/// Note a failed step.
fn check<T>(r: Result<T>, what: &str, problems: &mut Vec<String>) -> Option<T> {
    r.map_err(|e| problems.push(format!("{what}: {e}"))).ok()
}

fn build_people(people: &FakePeople, out: &mut Book) {
    for name in HUMANS {
        out.persons.insert(name, people.human(name));
    }
    out.persons.insert(AGENT, people.agent(AGENT));
    out.persons.insert(UNKNOWN, people.unknown(UNKNOWN));
    for (label, admin) in [
        ("Chair", false),
        ("Treasurer", false),
        ("Administrator", true),
    ] {
        out.roles.insert(label, people.add_role(label, admin));
    }
}

fn build_groups(s: &mut State, out: &mut Book) {
    for g in groups() {
        let art = if g.organization {
            GroupArt::Organization
        } else {
            GroupArt::Group
        };
        let visibility = if g.sensitive {
            Visibility::Members
        } else {
            Visibility::Everyone
        };
        let draft = GroupDraft {
            art,
            name: g.name.to_owned(),
            kind: Some(g.kind.to_owned()),
            description: None,
            default_mode: NestingMode::BottomUp,
            visibility,
            sensitive: g.sensitive,
            ai: g.ai,
            mirror: g.mirror,
        };
        if let Some(made) = check(s.create(draft), g.name, &mut out.problems) {
            out.ids.insert(g.name, made.id);
        }
    }
}

fn build_edges(s: &mut State, out: &mut Book) {
    for (child, parent, mode) in EDGES {
        let (Some(c), Some(p)) = (out.id(child), out.id(parent)) else {
            continue;
        };
        check(
            s.request_nesting(c, p, mode_of(mode)),
            child,
            &mut out.problems,
        );
    }
}

fn build_members(s: &mut State, out: &mut Book) {
    for (group, person, role) in MEMBERS {
        let (Some(g), Some(p)) = (out.id(group), out.person(person)) else {
            continue;
        };
        let role = role.and_then(|r| out.role(r));
        check(
            s.add_member(g, &p, role.as_ref()),
            person,
            &mut out.problems,
        );
    }
}

/// Rights, invitations of every state, one open nesting request, one group in the trash.
fn build_activity(s: &mut State, out: &mut Book) {
    let ids = |n: &str| out.id(n).unwrap_or_else(|| crate::ids::object_id(0));
    let fallback = s.asker.clone();
    let who = |n: &str| out.person(n).unwrap_or_else(|| fallback.clone());
    let (assoc, events, outreach, chiefs) = (
        ids("Association"),
        ids("Events team"),
        ids("Outreach team"),
        ids("Chiefs"),
    );
    let alice = who("Alice Chair");
    let mut problems = Vec::new();
    check(
        s.grant(events, &who("Carol Lead"), Right::Manage),
        "grant Carol",
        &mut problems,
    );
    check(
        s.grant(assoc, &who("Bob Deputy"), Right::AddMembers),
        "grant Bob",
        &mut problems,
    );
    check(
        s.grant(assoc, &group_ref(chiefs), Right::RemoveMembers),
        "grant Chiefs",
        &mut problems,
    );
    check(
        s.grant(outreach, &group_ref(ids("Assistants")), Right::Read),
        "grant Assistants",
        &mut problems,
    );
    let later = Some(Utc::from_millis(s.now.millis() + 30 * 24 * 3600 * 1000));
    check(
        s.invite(events, &who("Frank Stage"), None, later),
        "invite Frank",
        &mut problems,
    );
    let gina = check(
        s.invite(outreach, &who("Gina Guest"), None, None),
        "invite Gina",
        &mut problems,
    );
    let jonas = check(
        s.invite(events, &who("Jonas Intern"), None, None),
        "invite Jonas",
        &mut problems,
    );
    for (inv, name, answer) in [
        (gina, "Gina Guest", InvitationAnswer::Accept),
        (jonas, "Jonas Intern", InvitationAnswer::Decline),
    ] {
        if let Some(i) = inv {
            s.asker = who(name);
            check(
                s.answer_invitation(i.id, i.revision, answer),
                name,
                &mut problems,
            );
        }
    }
    s.asker = who("Carol Lead");
    check(
        s.request_nesting(outreach, events, NestingMode::Single),
        "request Carol",
        &mut problems,
    );
    s.asker = alice;
    if let Some(archive) = out.id("Archive") {
        let rev = s.alive(archive).map_or(Revision(1), |g| g.revision);
        check(s.delete(archive, rev), "delete Archive", &mut problems);
    }
    out.problems.extend(problems);
}

/// A directory with an old state: the trashed group still in it, a foreign account on Administrators.
fn build_mirror(out: &Book) -> FakeMirror {
    let mirror = FakeMirror::new();
    let blank = |g: ObjectId, name: &str| DirectoryGroup {
        group: g,
        name: name.to_owned(),
        members: Vec::new(),
        nested: Vec::new(),
        foreign: Vec::new(),
    };
    if let Some(admins) = out.id("Administrators") {
        mirror.seed(DirectoryGroup {
            foreign: vec![DirectoryAccount("legacy@kanidm.example.org".to_owned())],
            ..blank(admins, "Administrators")
        });
    }
    if let Some(archive) = out.id("Archive") {
        mirror.seed(blank(archive, "Archive"));
    }
    mirror
}

/// The fixed sample (the clock moves one second per write from [`crate::START_MS`]).
#[must_use]
pub fn sample() -> Sample {
    let people = Arc::new(FakePeople::new());
    let mut book = Book::default();
    build_people(&people, &mut book);
    let alice = book
        .person("Alice Chair")
        .unwrap_or_else(|| crate::ids::person(0));
    let groups = FakeGroups::with_people(Arc::clone(&people), alice);
    {
        let mut s = groups.lock();
        build_groups(&mut s, &mut book);
        build_edges(&mut s, &mut book);
        build_members(&mut s, &mut book);
        build_activity(&mut s, &mut book);
    }
    let mirror = build_mirror(&book);
    Sample {
        groups,
        mirror,
        people,
        book,
    }
}
