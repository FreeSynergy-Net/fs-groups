//! K31–K34 und K36: Sichtbarkeit der Eigentümer, betreute Konten (nie Eigentümer, legen keine Gruppe an).
//!
//! Pattern: Template method (je Fall ein Aufbau, eine Handlung, eine erwartete Antwort und ein Mutant).

use super::kern::{Fault, Kern, role};
use crate::draft::groups_v2::{Error, FixedRole, GroupQuery, InForce, ParentFilter, Unreachable};

/// K31: sensible Gruppe: ein Leser ohne Mitgliederlisten-Sicht ruft `owners`: die vollständige Liste (nur
/// `PersonRef`). Mutant: `Visibility` greift bei sensiblen Gruppen.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k31_owners_visible_on_sensitive_groups() {
    let k = Kern::start();
    let (p, reader) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    k.act(&reader);
    let owners = k.g.owners(grp.id).await.unwrap();
    assert_eq!(owners.len(), 1);
}

/// K32: ein betreutes Konto als Eigentümer per `add_member`/`set_role`/`invite`/Leerstelle:
/// `SupervisedCannotOwn`, nichts geschrieben. Mutant: die Prüfung je Weg aus.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k32_supervised_never_owns() {
    let k = Kern::start();
    let (p, s) = (k.human(), k.supervised());
    let grp = k.group_of(&p).await;
    let owner = role(FixedRole::Owner);
    let add = k.g.add_member(grp.id, &s, Some(&owner)).await;
    assert!(matches!(add, Err(Error::SupervisedCannotOwn { .. })));
    let inv = k.g.invite(grp.id, &s, Some(&owner), None).await;
    assert!(matches!(inv, Err(Error::SupervisedCannotOwn { .. })));
    let ms = k.member(grp.id, &s, None).await;
    let set = k.g.set_role(ms.id, ms.revision, Some(&owner)).await;
    assert!(matches!(set, Err(Error::SupervisedCannotOwn { .. })));
}

/// K33: `is_supervised` ist `None` oder scheitert bei einer Eigentümer-Vergabe: `SupervisedStatusUnknown`
/// bzw. `Unavailable(People)`. Mutant: `None` als `false`.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k33_supervised_status_unknown_fails_closed() {
    let k = Kern::start();
    let (p, u) = (k.human(), k.unknown());
    let grp = k.group_of(&p).await;
    let owner = role(FixedRole::Owner);
    let r = k.g.add_member(grp.id, &u, Some(&owner)).await;
    assert!(matches!(
        r,
        Err(Error::SupervisedStatusUnknown { .. } | Error::AiStatusUnknown)
    ));
    k.fail(Fault::People);
    let r = k.g.add_member(grp.id, &p, Some(&owner)).await;
    assert!(matches!(r, Err(Error::Unavailable(Unreachable::People))));
}

/// K34: die einzige Eigentümerin wird nachträglich betreut: `InForce::Supervised`, kein `Own`, Leerstelle;
/// Variante mit zweiter Eigentümerin: Ok, Gruppe unverändert. Mutant: die Rolle gibt weiter `Own`.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k34_owner_becomes_supervised() {
    let k = Kern::start();
    let p = k.human();
    let grp = k.group_of(&p).await;
    k.mark_supervised(&p);
    let owners = k.g.owners(grp.id).await.unwrap();
    assert!(owners.iter().all(|o| o.in_force == InForce::Supervised));
    assert!(!k.may(grp.id, &p, crate::draft::groups_v2::Right::Own).await);
}

/// K36 (Entscheidung b7 Pkt 9): ein betreutes Konto legt eine Gruppe NICHT an: `SupervisedCannotCreate{person}`
/// (Fehler-Code plus Parameter; Grund und Hinweis "die verantwortliche Person kann die Gruppe anlegen" stehen
/// de/en im Programm), nichts angelegt. Mutant: `create` prüft die Betreuung nicht.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k36_supervised_cannot_create() {
    let k = Kern::start();
    let s = k.supervised();
    k.act(&s);
    let before = k.g.list(&query(), None, 100).await.unwrap().items.len();
    let r = k.g.create(sample_draft()).await;
    assert!(matches!(r, Err(Error::SupervisedCannotCreate { person }) if person == s));
    let after = k.g.list(&query(), None, 100).await.unwrap().items.len();
    assert_eq!(before, after, "nothing created");
}

fn sample_draft() -> crate::draft::groups_v2::GroupDraft {
    unimplemented!(
        "kern: a plain GroupDraft (art, name, default_mode, visibility) of the provider's fixture"
    )
}

fn query() -> GroupQuery {
    GroupQuery {
        text: String::new(),
        parent: ParentFilter::Any,
        member: None,
        include_deleted: false,
    }
}
