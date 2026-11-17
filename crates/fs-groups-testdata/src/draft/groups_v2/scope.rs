//! Drei Verwaltungsstufen als Gruppen mit Geltungsbereich (ADR 0095, Antwort 100) und der Eingriff von oben.
//!
//! Pattern: Value Object (Bereich, Bereichsrecht, Eingriff) + Proxy (jede Handlung über einen Bereich läuft als
//! Eingriff durch dieselbe Prüfung und schreibt Protokoll; der Bereich selbst gibt nie Eigentum oder Verwaltung).
//!
//! Stufen: (a) `Scope::Project` (Projekt, Server, alle Gruppen, Vergabe der Bereiche), (b) `Scope::AllGroups`
//! (alle Gruppen, nicht Projekt und Server), (c) `Scope::Group(id)` = das v1-`Grant` an einer Gruppe
//! (Selbstverwaltung durch Eigentümer und Verwalter). Inhaber von (a)/(b) ist IMMER eine Gruppe, nie eine Person
//! und kein Sonderkonto; es gilt für deren DIREKTE, bekanntermaßen menschliche, wirksame Mitglieder (`InForce::Yes`),
//! nie für geerbte (ADR 0095 Pkt 15): der Bereich fließt über keine Kante.
//!
//! Regeln: `Scope::Group(_)` ist in `ScopedGrant` verboten (`Error::Invalid`; das bleibt das v1-`Grant`). Nur wer
//! über `Project` handelt, vergibt oder entzieht `ScopedGrant`; das Einrichten legt (a) und (b) an, die
//! einrichtende Person ist erste Eigentümerin beider (Einzelplatz: implizite Gruppe "Instanz", ADR 0090).
//! `reason` darf nicht leer sein (`Error::ReasonMissing`). Ein Bereich zählt NIE als Eigentum oder Verwaltung der
//! Zielgruppe, sonst griffe der Schutz der letzten (`LastOwner`, `LastManager`) nie.
//!
//! EINGRIFFSFRIST (Entscheidung b7 Pkt 11): ein Eingriff schließt von selbst nach einer Stunde
//! (`INTERVENTION_TTL_SECS_DEFAULT`); der Wert ist EINSTELLBAR: der Anbieter bekommt ihn als Parameter der
//! Instanz (Konfiguration, nie ein Parameter der Methode, nie ein Draht-Feld), die Uhr ist wie überall
//! Eigenschaft des Anbieters. Die berechnete Frist steht im Eingriff (`expires`), damit jede Lesende sie sieht.

use super::shared::{ObjectId, PersonRef, Revision, Utc};

/// Standard der Eingriffsfrist in Sekunden (eine Stunde); der Anbieter darf sie anders einstellen.
pub const INTERVENTION_TTL_SECS_DEFAULT: u32 = 3600;

/// Geltungsbereich eines Rechts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    /// (a) Projekt-Verwaltung.
    Project,
    /// (b) Gruppen-Verwaltung.
    AllGroups,
    /// (c) Selbstverwaltung: nur diese Gruppe (v1-`Grant`).
    Group(ObjectId),
}

/// Recht einer Admin-Gruppe im Bereich `Project` oder `AllGroups`. `holder` ist eine Gruppe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedGrant {
    pub id: ObjectId,
    pub revision: Revision,
    pub holder: ObjectId,
    pub scope: Scope,
    pub granted_by: PersonRef,
    pub granted: Utc,
}

/// Eingriff von (a) oder (b) in einer Gruppe: nur damit wirkt der Bereich dort. Gebunden an die Identität der
/// Verbindung (`actor` = fragende Person), nie an einen Parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intervention {
    pub id: ObjectId,
    pub group: ObjectId,
    pub actor: PersonRef,
    /// Das `ScopedGrant`, über das der Eingriff läuft.
    pub via: ObjectId,
    pub scope: Scope,
    /// Pflicht, nicht leer.
    pub reason: String,
    pub opened: Utc,
    /// Ende nach der Frist (`opened` + Einstellung des Anbieters); danach `InterventionRequired`.
    pub expires: Utc,
    /// Gesetzt, wenn früher geschlossen (`close_intervention`) oder nach Ablauf geschlossen.
    pub closed: Option<Utc>,
}
