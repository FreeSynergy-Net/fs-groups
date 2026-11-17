//! K18–K22: Pflichten, die v1 schon hat und der Kern 1a verletzt (A2 Kreis, A3 Spiegel, Herkunft, A8, A4).
//! Der Entwurf lässt sie mit dem Kern laufen (Probe-Plan Punkt 6).
//!
//! Pattern: Template method (je Fall ein Aufbau, eine Handlung, eine erwartete Antwort und ein Mutant).

use super::kern::{Fault, Kern, role};
use crate::draft::groups_v2::{Error, FixedRole, NestingMode, Right};

/// K18: `x->p` `BottomUp`, `y->p` `TopDown`, `y->x` `BottomUp`: `Cycle{path}` (Bogenrichtung, erste = letzte Gruppe), auch
/// beim Zustimmen, beim Moduswechsel `Single` -> `TopDown` und bei `restore` (Kante bleibt weg). Mutant: nur der Baum.
#[tokio::test]
#[ignore = "kern: needs the v2 provider (Kern 1a, Auflage A2)"]
async fn k18_cycle_in_the_member_graph() {
    let k = Kern::start();
    let h = k.human();
    let (x, y, p) = (
        k.group_of(&h).await,
        k.group_of(&h).await,
        k.group_of(&h).await,
    );
    k.g.request_nesting(x.id, p.id, NestingMode::BottomUp)
        .await
        .unwrap();
    k.g.request_nesting(y.id, p.id, NestingMode::TopDown)
        .await
        .unwrap();
    let r = k.g.request_nesting(y.id, x.id, NestingMode::BottomUp).await;
    let Err(Error::Cycle { path }) = r else {
        panic!("expected Cycle, got {r:?}")
    };
    assert_eq!(path.first(), path.last());
}

/// K19: je Modus ein Spiegelbild: `BottomUp` Kind in `nested` der Eltern, `TopDown` Eltern in `nested` des
/// Kindes, `Single` kein Bogen; `DirectoryGroup.members` nur direkte Mitglieder. Mutant: Teilbäume flach nach oben.
#[tokio::test]
#[ignore = "kern: needs the v2 provider (Kern 1a, Auflage A3; mirror plan of the fake)"]
async fn k19_mirror_per_mode() {
    let k = Kern::start();
    let _ = k.human();
    unimplemented!("one tree per mode, compare mirror_plan with the DirectoryGroup of each mode");
}

/// K20: Diamant mit zwei Quellen: je Quelle ein `Inherited`, kürzester Weg zuerst. Mutant: `also` verworfen.
#[tokio::test]
#[ignore = "kern: needs the v2 provider (Kern 1a, Auflage A1: `also` of the core maps to a second `Inherited`)"]
async fn k20_diamond_two_sources() {
    let k = Kern::start();
    let h = k.human();
    let (top, l, r, bottom) = (
        k.group_of(&h).await,
        k.group_of(&h).await,
        k.group_of(&h).await,
        k.group_of(&h).await,
    );
    for (c, p) in [
        (l.id, top.id),
        (r.id, top.id),
        (bottom.id, l.id),
        (bottom.id, r.id),
    ] {
        k.g.request_nesting(c, p, NestingMode::BottomUp)
            .await
            .unwrap();
    }
    let page = k.g.effective_members(top.id, None, 50).await.unwrap().items;
    assert!(
        page.iter()
            .all(|m| m.inherited.windows(2).all(|w| w[0] != w[1]))
    );
}

/// K21: Property-Test (A8) mit EIGENEM Orakel (Mengenmodell: Person, Rolle, `in_force`, betreut, Eingriff;
/// mehrere Einträge, `in_force=false`, Herabstufen). Invariante: wer Eigentümer oder Verwaltung hatte, behält
/// mindestens einen nach jeder angenommenen Handlung; jede Ablehnung hat ein Gegenbeispiel im Orakel.
/// Mutant: das Orakel ist `check_change` nachgebaut (verboten).
#[tokio::test]
#[ignore = "kern: needs the v2 provider; the oracle is its own set model, written with the provider"]
async fn k21_property_test_with_own_oracle() {
    let k = Kern::start();
    unimplemented!("random actions on {:?}, oracle = set model", k.human());
}

/// K22: `delete` parallel zu Herabstufen (`SlowWrite`): nie Löschen durch eine Person ohne Recht zum
/// Schreibzeitpunkt. Mutant: Prüfen vor der Sperre.
#[tokio::test]
#[ignore = "kern: needs the v2 provider (SlowGroups v2, Kern 1a, Auflage A4)"]
async fn k22_delete_races_demote() {
    let k = Kern::start();
    let (p, m) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let mm = k.member(grp.id, &m, Some(&role(FixedRole::Manager))).await;
    k.fail(Fault::SlowWrite);
    // Who asks is a property of the provider handle: the fixture gives one handle per person.
    let (as_p, as_m) = (k.as_person(&p), k.as_person(&m));
    let (_demote, r) = tokio::join!(
        as_p.set_role(mm.id, mm.revision, None),
        as_m.delete(grp.id, grp.revision)
    );
    assert!(r.is_err() || !k.may(grp.id, &m, Right::Manage).await);
}
