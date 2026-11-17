//! K11–K17 und Leerstelle: Übergabe, Eigentum nur über die Rolle, Leerstelle (inkl. Tombstone, Stufen (a)/(b)).
//!
//! Pattern: Template method (je Fall ein Aufbau, eine Handlung, eine erwartete Antwort und ein Mutant).

use super::kern::{Fault, Kern, role};
use crate::draft::groups_v2::{Error, FixedRole, HistoryKind, Keep, Missing, Right};

/// K11: Übergabe gelingt: `to` Eigentümer, `from` = `keep`, `OwnershipTransferred` im Protokoll. Mutant:
/// Schreibreihenfolge vertauscht (`FailingGroups` mit `AfterFirstWrite`: dann bliebe kein Eigentümer, nicht zwei).
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k11_transfer_succeeds_and_aborts_safely() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let mq = k.member(grp.id, &q, None).await;
    let from = k.g.owners(grp.id).await.unwrap().remove(0);
    let t =
        k.g.transfer_ownership(from.id, from.revision, mq.id, Keep::Manager)
            .await
            .unwrap();
    assert_eq!(t.to.role, Some(role(FixedRole::Owner)));
    assert_eq!(t.from.role, Some(role(FixedRole::Manager)));
    let h = k.g.history(grp.id, None, 50).await.unwrap().items;
    assert!(
        h.iter()
            .any(|e| matches!(e.kind, HistoryKind::OwnershipTransferred { .. }))
    );
    // Abbruch nach dem ersten Schreiben: zwei Eigentümer, nie keiner.
    let r = k.human();
    let mr = k.member(grp.id, &r, None).await;
    k.fail(Fault::AfterFirstWrite);
    let again = k.g.owners(grp.id).await.unwrap().remove(0);
    let _ =
        k.g.transfer_ownership(again.id, again.revision, mr.id, Keep::Member)
            .await;
    let left = k.g.owners(grp.id).await.unwrap().len();
    assert_ne!(left, 0, "an abort leaves two owners, never none");
}

/// K12: Übergabe mit alter Revision: `Changed`, nichts geschrieben. Mutant: `expected` ignoriert.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k12_transfer_stale_revision() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let mq = k.member(grp.id, &q, None).await;
    let from = k.g.owners(grp.id).await.unwrap().remove(0);
    let stale = crate::draft::groups_v2::shared::Revision(from.revision.0.wrapping_sub(1));
    let r =
        k.g.transfer_ownership(from.id, stale, mq.id, Keep::Member)
            .await;
    assert!(matches!(r, Err(Error::Changed { .. })));
}

/// K13: Übergabe an Nichtmitglied, KI, Unbekannte, schon Eigentümer, Betreute: `NotFound(Membership)`,
/// `AgentNotAdmin`, `AiStatusUnknown`, `AlreadyThere`, `SupervisedCannotOwn`. Mutant: je eine Prüfung aus.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k13_transfer_targets() {
    let k = Kern::start();
    let (p, a, u, s) = (k.human(), k.agent(), k.unknown(), k.supervised());
    let grp = k.group_of(&p).await;
    let from = k.g.owners(grp.id).await.unwrap().remove(0);
    let go = |to| {
        k.g.transfer_ownership(from.id, from.revision, to, Keep::Member)
    };
    let foreign = crate::draft::groups_v2::shared::ObjectId([9; 16]);
    assert!(matches!(
        go(foreign).await,
        Err(Error::NotFound(Missing::Membership(_)))
    ));
    let (ma, mu, ms) = (
        k.member(grp.id, &a, None).await,
        k.member(grp.id, &u, None).await,
        k.member(grp.id, &s, None).await,
    );
    assert!(matches!(go(ma.id).await, Err(Error::AgentNotAdmin)));
    assert!(matches!(go(mu.id).await, Err(Error::AiStatusUnknown)));
    assert!(matches!(
        go(ms.id).await,
        Err(Error::SupervisedCannotOwn { .. })
    ));
    assert!(matches!(go(from.id).await, Err(Error::AlreadyThere(_))));
}

/// K14: Verwalter ohne `Own` übergibt, stuft Eigentümer herab, entfernt ihn, löscht die Gruppe mit Eigentümer:
/// `Forbidden{Own}`. Mutant: `Manage` reicht.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k14_manage_is_not_own() {
    let k = Kern::start();
    let (p, m) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let mm = k.member(grp.id, &m, Some(&role(FixedRole::Manager))).await;
    let own = k.g.owners(grp.id).await.unwrap().remove(0);
    k.act(&m);
    let forb = |r: Result<_, Error>| {
        matches!(
            r,
            Err(Error::Forbidden {
                right: Right::Own,
                ..
            })
        )
    };
    assert!(forb(
        k.g.transfer_ownership(own.id, own.revision, mm.id, Keep::Member)
            .await
            .map(|_| ())
    ));
    assert!(forb(
        k.g.set_role(own.id, own.revision, None).await.map(|_| ())
    ));
    assert!(forb(k.g.remove_member(own.id, own.revision).await));
    assert!(forb(k.g.delete(grp.id, grp.revision).await.map(|_| ())));
}

/// K15: `grant(Own)`: `OwnViaRoleOnly`. Mutant: `Own` als Grant erlaubt.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k15_own_only_via_role() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let r = k.g.grant(grp.id, &q, Right::Own).await;
    assert!(matches!(r, Err(Error::OwnViaRoleOnly)));
}

/// K16: Leerstelle (kein wirksamer Eigentümer): ein Verwalter benennt: `Forbidden{Own}`; (a) UND (b) per
/// Eingriff benennen: Ok, beide Stufen (Entscheidung b7 Pkt 8), jeweils mit Eingriff im Protokoll. Mutant:
/// `Manage` reicht, oder nur (a) darf.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k16_vacancy_filled_by_a_or_b() {
    let k = Kern::start();
    let (m, c1, c2) = (k.human(), k.human(), k.human());
    let grp = k.legacy_group(&m).await;
    let owner = Some(role(FixedRole::Owner));
    k.act(&m);
    let r = k.g.add_member(grp.id, &c1, owner.as_ref()).await;
    assert!(matches!(
        r,
        Err(Error::Forbidden {
            right: Right::Own,
            ..
        })
    ));
    let (pa, ga, pb, gb) = k.scopes().await;
    for (who, via, cand) in [(pa, ga, &c1), (pb, gb, &c2)] {
        k.act(&who);
        let iv =
            k.g.open_intervention(grp.id, via, "fill the vacancy")
                .await
                .unwrap();
        assert!(k.g.add_member(grp.id, cand, owner.as_ref()).await.is_ok());
        let h = k.g.history(grp.id, None, 50).await.unwrap().items;
        assert!(h.iter().any(|e| e.intervention == Some(iv.id)));
        k.g.close_intervention(iv.id).await.unwrap();
    }
}

/// K16b (Entscheidung b7 Pkt 7): Eigentum einer gelöschten Person (Tombstone) ist Leerstelle: der Eintrag bleibt
/// (`InForce::Tombstone`, kein `Own`), ein Verwalter allein füllt nicht, (a)/(b) per Eingriff schon.
/// Mutant: ein Tombstone-Eigentümer zählt weiter als wirksam.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k16b_tombstone_owner_is_vacancy() {
    let k = Kern::start();
    let (gone, m) = (k.human(), k.human());
    let grp = k.group_of(&gone).await;
    let _ = k.member(grp.id, &m, Some(&role(FixedRole::Manager))).await;
    k.delete_person(&gone);
    let owners = k.g.owners(grp.id).await.unwrap();
    assert!(
        owners
            .iter()
            .all(|o| o.in_force == crate::draft::groups_v2::InForce::Tombstone)
    );
    assert!(!k.may(grp.id, &gone, Right::Own).await);
    k.act(&m);
    let r =
        k.g.add_member(grp.id, &k.human(), Some(&role(FixedRole::Owner)))
            .await;
    assert!(matches!(
        r,
        Err(Error::Forbidden {
            right: Right::Own,
            ..
        })
    ));
}

/// K17: `Some(ROLE_MEMBER)` schreiben, lesen: `None`; die Altformen der Migration. Mutant: Normalisierung aus.
#[tokio::test]
#[ignore = "kern: needs the v2 provider (the pure round trip of the role values runs in `pure`)"]
async fn k17_member_role_is_normalized() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let m = k.member(grp.id, &q, Some(&role(FixedRole::Member))).await;
    assert_eq!(m.role, None);
}
