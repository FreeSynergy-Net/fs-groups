//! Die Fassung des Anbieters für die Fälle: Personen jeder Art, Fehler einspritzen, Uhr, Bereichsgruppen.
//!
//! Pattern: Strategy / Test fixture (jeder Anbieter sagt, wie man ihn in die Zustände der Fälle bringt; hier
//! nur die Form, ohne Anbieter: alles `unimplemented!`, bis der Kern da ist).

use crate::draft::groups_v2::shared::{BasicValue, ObjectId, PersonRef};
use crate::draft::groups_v2::{Group, Groups, Membership, Right, RoleRef};

/// Grund für `unimplemented!`: ein ignorierter Fall darf nie still grün werden.
const KERN: &str = "kern: the v2 provider (FakeGroups v2 / LocalGroups) does not exist yet";

/// Was der Anbieter an Fehlern einspritzen können muss (wie `FailingGroups`/`SlowGroups` in v1).
#[derive(Clone, Copy, Debug)]
pub(super) enum Fault {
    /// `direct()` (Vorher-Stand lesen) scheitert (K8).
    ReadBefore,
    /// `PersonFacts` nicht erreichbar (K9, K33).
    People,
    /// Abbruch nach dem ersten Schreiben (K11, Reihenfolge der Übergabe).
    AfterFirstWrite,
    /// Schreiben langsam, damit ein zweiter Aufruf dazwischen kommt (K22).
    SlowWrite,
}

/// Der Anbieter unter Test plus die Dinge, die der Trait nicht kann.
pub(super) struct Kern {
    pub(super) g: Box<dyn Groups>,
}

impl Kern {
    pub(super) fn start() -> Self {
        unimplemented!("{KERN}")
    }
    /// Neue Person der Art (Mensch, KI, unbekannt, betreut, gelöscht).
    pub(super) fn human(&self) -> PersonRef {
        unimplemented!("{KERN}")
    }
    pub(super) fn agent(&self) -> PersonRef {
        unimplemented!("{KERN}")
    }
    pub(super) fn unknown(&self) -> PersonRef {
        unimplemented!("{KERN}")
    }
    pub(super) fn supervised(&self) -> PersonRef {
        unimplemented!("{KERN}")
    }
    /// Ein Anbieter-Handle, das als diese Person fragt (für Fälle mit zwei gleichzeitigen Personen).
    pub(super) fn as_person(&self, _who: &PersonRef) -> Box<dyn Groups> {
        unimplemented!("{KERN}")
    }
    /// Die fragende Person wechseln (Eigenschaft des Anbieters).
    pub(super) fn act(&self, _who: &PersonRef) {
        unimplemented!("{KERN}")
    }
    /// Markierung nachträglich ändern (KI, betreut, gelöscht).
    pub(super) fn mark_supervised(&self, _who: &PersonRef) {
        unimplemented!("{KERN}")
    }
    pub(super) fn mark_agent(&self, _who: &PersonRef) {
        unimplemented!("{KERN}")
    }
    pub(super) fn delete_person(&self, _who: &PersonRef) {
        unimplemented!("{KERN}")
    }
    pub(super) fn fail(&self, _fault: Fault) {
        unimplemented!("{KERN}")
    }
    /// Die Uhr des Anbieters vorstellen und die Eingriffsfrist einstellen (Entscheidung b7 Pkt 11).
    pub(super) fn tick(&self, _secs: u32) {
        unimplemented!("{KERN}")
    }
    pub(super) fn set_intervention_ttl(&self, _secs: u32) {
        unimplemented!("{KERN}")
    }
    /// Gruppe von `owner` angelegt (er ist Eigentümer).
    pub(super) async fn group_of(&self, _owner: &PersonRef) -> Group {
        unimplemented!("{KERN}")
    }
    /// Altgruppe ohne Eigentümer (Altdaten, Migration): `manager` ist der einzige Verwalter.
    pub(super) async fn legacy_group(&self, _manager: &PersonRef) -> Group {
        unimplemented!("{KERN}")
    }
    /// Direkte Mitgliedschaft mit Rolle (hinzugefügt von der gerade fragenden Person).
    pub(super) async fn member(
        &self,
        _g: ObjectId,
        _p: &PersonRef,
        _r: Option<&RoleRef>,
    ) -> Membership {
        unimplemented!("{KERN}")
    }
    /// Admin-Gruppen (a) und (b) mit ihrem `ScopedGrant`, je mit einem menschlichen Mitglied; Ergebnis:
    /// `(person_a, grant_a, person_b, grant_b)`.
    pub(super) async fn scopes(&self) -> (PersonRef, ObjectId, PersonRef, ObjectId) {
        unimplemented!("{KERN}")
    }
    /// Das Recht der Person an der Gruppe, wie `may_do` es sagt.
    pub(super) async fn may(&self, g: ObjectId, who: &PersonRef, right: Right) -> bool {
        self.g.may_do(g, who, right).await.is_ok_and(|d| d.allowed)
    }
}

/// Die feste Rolle als `RoleRef`.
pub(super) fn role(r: crate::draft::groups_v2::FixedRole) -> RoleRef {
    RoleRef(crate::draft::groups_v2::shared::Reference::parse(r.iri()).expect("fixed role IRI"))
}
