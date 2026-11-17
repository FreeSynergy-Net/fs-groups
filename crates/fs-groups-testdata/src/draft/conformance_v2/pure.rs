//! Fälle, die ohne Anbieter laufen: K17 (feste Rollen, Rückweg), Fähigkeitsliste, byte-gleiche v1-Dateien, ein
//! einziges `MAX_NESTING_DEPTH`, Eingriffsfrist-Standard.
//!
//! Pattern: Template method (feste Fälle; hier gegen die Werte des Ordners selbst).

use crate::draft::groups_v2::shared::{BasicValue, Reference};
use crate::draft::groups_v2::{
    CAPABILITIES, FixedRole, INTERVENTION_TTL_SECS_DEFAULT, ROLE_LEGACY_BASE, Right, RoleRef,
};

/// Die fünf Dateien, die der Entwurf byte-gleich aus v1 übernimmt.
const SAME: [(&str, &str, &str); 5] = [
    (
        "shared.rs",
        include_str!("../../groups_v1/shared.rs"),
        include_str!("../groups_v2/shared.rs"),
    ),
    (
        "group.rs",
        include_str!("../../groups_v1/group.rs"),
        include_str!("../groups_v2/group.rs"),
    ),
    (
        "nesting.rs",
        include_str!("../../groups_v1/nesting.rs"),
        include_str!("../groups_v2/nesting.rs"),
    ),
    (
        "mirror.rs",
        include_str!("../../groups_v1/mirror.rs"),
        include_str!("../groups_v2/mirror.rs"),
    ),
    (
        "provided.rs",
        include_str!("../../groups_v1/provided.rs"),
        include_str!("../groups_v2/provided.rs"),
    ),
];

fn rref(iri: &str) -> RoleRef {
    RoleRef(Reference::parse(iri).expect("a role IRI"))
}

/// K17: jede feste Rolle geht hin und zurück; `None` ist `Member`; Alt-Rolle und Fremdes geben kein Recht.
/// Mutant: `from_ref` vergleicht nur die Rolle `Owner` (dann fällt `Manager` heraus) oder wertet `None` als `None`.
#[test]
fn k17_fixed_roles_round_trip() {
    for r in [FixedRole::Member, FixedRole::Manager, FixedRole::Owner] {
        assert_eq!(FixedRole::from_ref(Some(&rref(r.iri()))), Some(r), "{r:?}");
    }
    assert_eq!(
        FixedRole::from_ref(None),
        Some(FixedRole::Member),
        "plain member is role None"
    );
    let legacy = rref(&format!("{ROLE_LEGACY_BASE}admin"));
    assert_eq!(
        FixedRole::from_ref(Some(&legacy)),
        None,
        "a legacy role gives no right"
    );
    assert_eq!(
        FixedRole::from_ref(Some(&rref("https://freesynergy.net/ns/roles/ai"))),
        None
    );
}

/// Die drei IRIs sind verschieden und stehen unter der Basis von fs-people.
/// Mutant: zwei Rollen mit gleicher IRI.
#[test]
fn fixed_role_iris_are_distinct_and_flat() {
    let iris = [
        FixedRole::Member.iri(),
        FixedRole::Manager.iri(),
        FixedRole::Owner.iri(),
    ];
    assert!(
        iris.iter()
            .all(|i| i.starts_with("https://freesynergy.net/ns/roles/"))
    );
    assert!(iris[0] != iris[1] && iris[1] != iris[2] && iris[0] != iris[2]);
}

/// 14 Fähigkeiten, jede einmal, mit `member.own` und `group.intervene`; `Own` ist die höchste Stufe.
/// Mutant: eine Fähigkeit doppelt oder vergessen.
#[test]
fn capabilities_are_14_and_unique() {
    assert_eq!(CAPABILITIES.len(), 14);
    let mut sorted = CAPABILITIES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 14);
    assert!(CAPABILITIES.contains(&"member.own") && CAPABILITIES.contains(&"group.intervene"));
    assert!(Right::Own > Right::Manage);
}

/// Die fünf Dateien sind byte-gleich v1 (Probe-Beleg zusätzlich per `diff` und Ordner-Hash).
/// Mutant: eine der fünf Dateien in v2 verändert.
#[test]
fn five_files_are_byte_equal_to_v1() {
    for (name, v1, v2) in SAME {
        assert!(v1 == v2, "{name} differs from groups_v1");
    }
}

/// Es gibt genau eine Konstante `MAX_NESTING_DEPTH` (in `nesting.rs`), kein Duplikat in v2.
/// Mutant: ein zweites `const MAX_NESTING_DEPTH` in `errors.rs` oder `scope.rs`.
#[test]
fn exactly_one_max_nesting_depth() {
    let all = [
        include_str!("../groups_v2/api.rs"),
        include_str!("../groups_v2/consumed.rs"),
        include_str!("../groups_v2/errors.rs"),
        include_str!("../groups_v2/members.rs"),
        include_str!("../groups_v2/mod.rs"),
        include_str!("../groups_v2/nesting.rs"),
        include_str!("../groups_v2/ownership.rs"),
        include_str!("../groups_v2/rights.rs"),
        include_str!("../groups_v2/scope.rs"),
    ];
    let n: usize = all
        .iter()
        .map(|s| s.matches("const MAX_NESTING_DEPTH").count())
        .sum();
    assert_eq!(n, 1);
}

/// Entscheidung b7 Pkt 11: Standardfrist eine Stunde. Mutant: andere Zahl.
#[test]
fn intervention_ttl_default_is_one_hour() {
    assert_eq!(INTERVENTION_TTL_SECS_DEFAULT, 3600);
}

/// `Groups` ist als `dyn` nutzbar (Probe-Plan: Anbieter als `dyn Groups`); Kompilierfall.
#[test]
fn groups_is_object_safe() {
    let _: Option<&dyn crate::draft::groups_v2::Groups> = None;
}
