//! K1–K10: der Schutz der letzten (`LastOwner`, `LastManager`) und sein fail-closed.
//!
//! Pattern: Template method (je Fall ein Aufbau, eine Handlung, eine erwartete Antwort und ein Mutant).

use super::kern::{Fault, Kern, role};
use crate::draft::groups_v2::{Error, FixedRole, GuardWhy, Right, Unreachable};

/// K1: einzige Eigentümerin tritt aus: `LastOwner{Leave}`, nichts geschrieben. Mutant: Schutz aus.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k01_sole_owner_leaves() {
    let k = Kern::start();
    let p = k.human();
    let grp = k.group_of(&p).await;
    let own = k.g.owners(grp.id).await.unwrap().remove(0);
    let r = k.g.remove_member(own.id, own.revision).await;
    assert!(matches!(
        r,
        Err(Error::LastOwner {
            why: GuardWhy::Leave,
            ..
        })
    ));
    assert_eq!(
        k.g.owners(grp.id).await.unwrap().len(),
        1,
        "nothing written"
    );
}

/// K2: einzige Eigentümerin per `set_role` zum Verwalter: `LastOwner{Demote}`. Mutant: nur `remove` geprüft.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k02_sole_owner_demoted() {
    let k = Kern::start();
    let p = k.human();
    let grp = k.group_of(&p).await;
    let own = k.g.owners(grp.id).await.unwrap().remove(0);
    let r =
        k.g.set_role(own.id, own.revision, Some(&role(FixedRole::Manager)))
            .await;
    assert!(matches!(
        r,
        Err(Error::LastOwner {
            why: GuardWhy::Demote,
            ..
        })
    ));
}

/// K3: zwei Eigentümer, einer tritt aus: Ok. Mutant: "mindestens 2" statt "mindestens 1".
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k03_two_owners_one_leaves() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let _ = k.member(grp.id, &q, Some(&role(FixedRole::Owner))).await;
    let own =
        k.g.owners(grp.id)
            .await
            .unwrap()
            .into_iter()
            .find(|m| m.person == q)
            .unwrap();
    k.act(&q);
    assert!(k.g.remove_member(own.id, own.revision).await.is_ok());
}

/// K4: Altgruppe ohne Eigentümer, der letzte Verwalter wird entfernt: `LastManager{Remove}`.
/// Mutant: der Owner-Zweig ohne Else (ohne Eigentümer gar keine Prüfung).
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k04_legacy_group_last_manager() {
    let k = Kern::start();
    let (m, h) = (k.human(), k.human());
    let grp = k.legacy_group(&m).await;
    // h may remove members but does not manage: removing the only manager m is refused.
    k.g.grant(grp.id, &h, Right::RemoveMembers).await.unwrap();
    let all = k.g.members(grp.id, None, 10).await.unwrap().items;
    let last = all.iter().find(|x| x.person == m).unwrap();
    k.act(&h);
    let r = k.g.remove_member(last.id, last.revision).await;
    assert!(matches!(
        r,
        Err(Error::LastManager {
            why: GuardWhy::Remove,
            ..
        })
    ));
}

/// K5: `Manage` nur per `grant`; `revoke_grant`: `LastManager{RevokeGrant}`. Mutant: Grants nicht gezählt.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k05_last_grant_revoked() {
    let k = Kern::start();
    let (m, h) = (k.human(), k.human());
    let grp = k.legacy_group(&m).await;
    let gr = k.g.grant(grp.id, &h, Right::Manage).await.unwrap();
    let mm = k.g.members(grp.id, None, 10).await.unwrap().items.remove(0);
    k.g.remove_member(mm.id, mm.revision).await.unwrap();
    let r = k.g.revoke_grant(gr.id, gr.revision).await;
    assert!(matches!(
        r,
        Err(Error::LastManager {
            why: GuardWhy::RevokeGrant,
            ..
        })
    ));
}

/// K6: den letzten Verwalter sperren: `LastManager{Block}`. Mutant: Block ungeprüft.
/// Offene Frage: v2 hat keine Methode `block` (nur die Begründung `GuardWhy::Block`); der Fall wartet auf den Kern.
#[tokio::test]
#[ignore = "kern: no `block` method in the contract yet (GuardWhy::Block is reserved); needs the v2 provider"]
async fn k06_last_manager_blocked() {
    let _ = Kern::start();
    unimplemented!("block is a person state in fs-people; which method reaches the guard is open");
}

/// K7: eine Person mit zwei Einträgen (Verwalter und Mitglied), der Verwalter-Eintrag fällt weg: `LastManager`.
/// Mutant: Zählen je Eintrag statt je Person.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k07_one_person_two_entries() {
    let k = Kern::start();
    let m = k.human();
    let grp = k.legacy_group(&m).await;
    let _ = k.member(grp.id, &m, None).await;
    let all = k.g.members(grp.id, None, 10).await.unwrap().items;
    let mgr = all
        .iter()
        .find(|x| x.person == m && x.role.is_some())
        .unwrap();
    let r = k.g.remove_member(mgr.id, mgr.revision).await;
    assert!(matches!(r, Err(Error::LastManager { .. })));
}

/// K8: `direct()` scheitert beim Entfernen: Speicherfehler, nichts geschrieben. Mutant: `if let Ok(before)`.
#[tokio::test]
#[ignore = "kern: needs the v2 provider (FailingGroups v2)"]
async fn k08_read_before_fails() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let mq = k.member(grp.id, &q, Some(&role(FixedRole::Manager))).await;
    k.fail(Fault::ReadBefore);
    let r = k.g.remove_member(mq.id, mq.revision).await;
    assert!(
        matches!(r, Err(Error::Storage(_))),
        "refused, not let through: {r:?}"
    );
}

/// K9: `PersonFacts` nicht erreichbar: `Unavailable(People)`. Mutant: Fehler verschluckt.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k09_people_unreachable() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let mq = k.member(grp.id, &q, Some(&role(FixedRole::Manager))).await;
    k.fail(Fault::People);
    let r = k.g.remove_member(mq.id, mq.revision).await;
    assert!(matches!(r, Err(Error::Unavailable(Unreachable::People))));
}

/// K10: alle Verwalter `AiStatusUnknown`, einen entfernen: `LastManager`. Mutant: im Vorher nur `Yes` gezählt.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k10_all_managers_unknown() {
    let k = Kern::start();
    let (m, u) = (k.human(), k.unknown());
    let grp = k.legacy_group(&m).await;
    let mu = k.member(grp.id, &u, Some(&role(FixedRole::Manager))).await;
    k.fail(Fault::People);
    let r = k.g.remove_member(mu.id, mu.revision).await;
    assert!(matches!(
        r,
        Err(Error::LastManager { .. } | Error::Unavailable(_))
    ));
}
