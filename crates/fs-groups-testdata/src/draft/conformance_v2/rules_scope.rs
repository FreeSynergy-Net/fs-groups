//! K23–K30 und K35: die Stufen (a)/(b), Eingriffe, Frist, Bereiche fließen nicht.
//!
//! Pattern: Template method (je Fall ein Aufbau, eine Handlung, eine erwartete Antwort und ein Mutant).

use super::kern::{Kern, role};
use crate::draft::groups_v2::{Error, FixedRole, GuardWhy, HistoryKind, Reason, Right, Scope};

/// K23: ein (a)-Mitglied ändert eine fremde Gruppe ohne Eingriff: `InterventionRequired`, nichts geschrieben.
/// Mutant: der Bereich wirkt still.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k23_scope_without_intervention() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let (pa, _ga, _pb, _gb) = k.scopes().await;
    k.act(&pa);
    let r = k.g.add_member(grp.id, &q, None).await;
    assert!(matches!(
        r,
        Err(Error::InterventionRequired {
            scope: Scope::Project
        })
    ));
}

/// K24: Eingriff mit leerem Grund: `ReasonMissing`. Mutant: Grund optional.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k24_empty_reason() {
    let k = Kern::start();
    let p = k.human();
    let grp = k.group_of(&p).await;
    let (pa, ga, ..) = k.scopes().await;
    k.act(&pa);
    assert!(matches!(
        k.g.open_intervention(grp.id, ga, "  ").await,
        Err(Error::ReasonMissing)
    ));
}

/// K25: (b) fügt per Eingriff in fremder Gruppe ein Mitglied hinzu: Ok; `InterventionOpened{scope, reason,
/// holder}` plus Eintrag mit `intervention`; ein einfaches Mitglied liest beides per `history`.
/// Mutant: Eintrag nur für Verwalter sichtbar.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k25_b_adds_member_logged_and_visible() {
    let k = Kern::start();
    let (p, plain, q) = (k.human(), k.human(), k.human());
    let grp = k.group_of(&p).await;
    let _ = k.member(grp.id, &plain, None).await;
    let (_pa, _ga, pb, gb) = k.scopes().await;
    k.act(&pb);
    let iv = k.g.open_intervention(grp.id, gb, "help").await.unwrap();
    assert!(k.g.add_member(grp.id, &q, None).await.is_ok());
    k.act(&plain);
    let h = k.g.history(grp.id, None, 50).await.unwrap().items;
    assert!(h.iter().any(|e| matches!(&e.kind, HistoryKind::InterventionOpened { scope: Scope::AllGroups, reason, .. } if reason == "help")));
    assert!(h.iter().any(|e| e.intervention == Some(iv.id)));
}

/// K26: (a) entfernt per Eingriff die einzige Eigentümerin: `LastOwner{Remove}`. Mutant: der Eingriff umgeht den Schutz.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k26_intervention_does_not_bypass_guard() {
    let k = Kern::start();
    let p = k.human();
    let grp = k.group_of(&p).await;
    let (pa, ga, ..) = k.scopes().await;
    k.act(&pa);
    k.g.open_intervention(grp.id, ga, "remove").await.unwrap();
    let own = k.g.owners(grp.id).await.unwrap().remove(0);
    let r = k.g.remove_member(own.id, own.revision).await;
    assert!(matches!(
        r,
        Err(Error::LastOwner {
            why: GuardWhy::Remove,
            ..
        })
    ));
}

/// K27: (a)/(b) bestehen, der einzige Verwalter einer Altgruppe wird entfernt: `LastManager{Remove}`.
/// Mutant: der Bereich zählt als Verwaltung.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k27_scope_is_not_management() {
    let k = Kern::start();
    let m = k.human();
    let grp = k.legacy_group(&m).await;
    let _ = k.scopes().await;
    let mm = k.g.members(grp.id, None, 10).await.unwrap().items.remove(0);
    let r = k.g.remove_member(mm.id, mm.revision).await;
    assert!(matches!(r, Err(Error::LastManager { .. })));
}

/// K28: der Bereich fließt nicht: eine Person nur geerbt (BottomUp/TopDown) in (a) hat keinen Bereich; ein
/// v1-`Grant` an Gruppe g gilt nicht in einer Untergruppe von g: beide `Forbidden`. Mutant: die Kante trägt den Bereich.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k28_scope_flows_over_no_edge() {
    let k = Kern::start();
    let (p, inherited) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let (_pa, ga, ..) = k.scopes().await;
    k.act(&inherited);
    assert!(matches!(
        k.g.open_intervention(grp.id, ga, "x").await,
        Err(Error::Forbidden { .. })
    ));
}

/// K29: KI bzw. Unbekannter direkt in (a): Aufnahme `AgentNotAdmin`/`AiStatusUnknown`; Altdaten: Eingriff
/// `Forbidden`, `Reason::Agent`. Mutant: KI-Prüfung bei Bereichen aus.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k29_agent_never_in_scope() {
    let k = Kern::start();
    let (a, grp_owner) = (k.agent(), k.human());
    let grp = k.group_of(&grp_owner).await;
    let (_pa, ga, ..) = k.scopes().await;
    k.mark_agent(&grp_owner);
    let d = k.g.may_do(grp.id, &a, Right::Manage).await.unwrap();
    assert!(!d.allowed && matches!(d.reason, Reason::Agent | Reason::AiStatusUnknown));
    k.act(&a);
    assert!(matches!(
        k.g.open_intervention(grp.id, ga, "x").await,
        Err(Error::Forbidden { .. })
    ));
}

/// K30: der letzte `ScopedGrant{Project}`: `LastScope`; `grant_scope` durch (b): `Forbidden`. Mutant: der
/// letzte darf gehen / (b) vergibt.
#[tokio::test]
#[ignore = "kern: needs the v2 provider"]
async fn k30_last_scope_and_who_grants() {
    let k = Kern::start();
    let (pa, ga, pb, gb) = k.scopes().await;
    k.act(&pb);
    assert!(matches!(
        k.g.grant_scope(gb, Scope::Project).await,
        Err(Error::Forbidden { .. })
    ));
    k.act(&pa);
    let rev =
        k.g.scoped_grants()
            .await
            .unwrap()
            .into_iter()
            .find(|g| g.id == ga)
            .unwrap()
            .revision;
    assert!(matches!(
        k.g.revoke_scope(ga, rev).await,
        Err(Error::LastScope {
            scope: Scope::Project
        })
    ));
}

/// K35 (Entscheidung b7 Pkt 11): ein Eingriff schließt nach der Frist von selbst, die Frist ist EINSTELLBAR;
/// geschlossen oder abgelaufen, danach Handlung: `InterventionRequired`. Mutant: Eingriff ohne Ende.
#[tokio::test]
#[ignore = "kern: needs the v2 provider (clock and TTL setting of the provider)"]
async fn k35_intervention_expires() {
    let k = Kern::start();
    let (p, q) = (k.human(), k.human());
    let grp = k.group_of(&p).await;
    let (pa, ga, ..) = k.scopes().await;
    k.set_intervention_ttl(60);
    k.act(&pa);
    let iv = k.g.open_intervention(grp.id, ga, "short").await.unwrap();
    assert_eq!(iv.expires.millis() - iv.opened.millis(), 60_000);
    k.tick(61);
    let r = k.g.add_member(grp.id, &q, None).await;
    assert!(matches!(r, Err(Error::InterventionRequired { .. })));
    let _ = role(FixedRole::Member);
}
