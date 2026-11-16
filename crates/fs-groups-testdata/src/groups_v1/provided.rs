//! ANGEBOTEN für andere Programme: wie Kalender, Tasks, Files und Surveys einen `PersonRef { art: Group |
//! Organization }` auflösen (ADR 0085 §3: Teilnehmer oder Zuständige können Gruppen sein). Schmal gehalten
//! (Interface Segregation): wer nur Mitglieder braucht, hängt nicht am ganzen `Groups`.
//!
//! Ebenso `RoleBook` für fs-people (zeigt Rollen einer Person nur an, Leitquelle bleibt fs-groups; Entscheidung
//! 21: das Rollenbuch war von fs-people verbraucht, jetzt bietet fs-groups es an) und für Auswahllisten.
//!
//! Pattern: Facade (schmale Sicht auf `Groups` bzw. `Roles` für Verbraucher).

use super::shared::PersonRef;
use super::{EffectiveRole, GroupArt, Result, RoleRef, RoleScope};
use async_trait::async_trait;

/// Name und Art einer Gruppe für die Anzeige beim Verbraucher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupCard {
    pub group: PersonRef,
    pub name: String,
    pub art: GroupArt,
    /// `None`, wenn die fragende Person die Mitglieder nicht sehen darf.
    pub effective_members: Option<u32>,
    /// `true`, wenn die Gruppe im Papierkorb liegt (Anzeige "gelöschte Gruppe").
    pub deleted: bool,
}

/// Gruppen auflösen. Anbieter: `LocalGroups`/`RemoteGroups` (Facade über `Groups`). Der Kalender ruft es zur
/// Zeit der Frage (frei/belegt, Versand einer Einladung), nie gespeichert: der Termin hält den Gruppenverweis.
#[async_trait]
pub trait GroupMembers: Send + Sync {
    /// Name und Art; `None`, wenn es die Gruppe nicht gibt.
    async fn describe(&self, group: &PersonRef) -> Result<Option<GroupCard>>;
    /// Wirksame Mitglieder (Regel `nesting.rs`, alle Modi; je einmal, Art `Person`/`Profile`), jetzt.
    /// `Forbidden`, wenn die fragende Person sie nicht sehen darf (außer: KI und Admin-/sensible Gruppe, dann
    /// `NotFound`, `nesting.rs`); `WrongArt` für einen Verweis auf eine Person.
    async fn members_of(&self, group: &PersonRef) -> Result<Vec<PersonRef>>;
    /// Gruppen, in denen `person` wirksam Mitglied ist (Filter "meine Gruppen" in Tasks, Kalender); nur
    /// Gruppen, die die fragende Person sehen darf: sensible Gruppen laufen nie über Kalender oder Tasks heraus; für
    /// eine KI fehlen Admin- und sensible Gruppen (`members_of` und `describe` dort: `NotFound` bzw. `None`).
    async fn memberships_of(&self, person: &PersonRef) -> Result<Vec<PersonRef>>;
}

/// Eine Rolle für Anzeige und Auswahl (W3C `org:Role`, ADR 0052).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleInfo {
    pub role: RoleRef,
    /// Claim-Wert ohne Präfix (`Role::key`).
    pub key: String,
    /// Anzeigename (`Role::name`).
    pub label: String,
    pub scope: RoleScope,
    /// Admin-wirksam (selbst oder über die Kette, `Protection::admin`): nie für KI oder unbekannte Personen.
    pub admin: bool,
    pub sensitive: bool,
    pub ai: bool,
}

/// Rollenbuch. Anbieter: `LocalGroups`/`RemoteGroups` (Facade über `Roles`); Sichtbarkeit wie `Roles` (eine KI
/// sieht keine admin-wirksamen und keine sensiblen Rollen).
#[async_trait]
pub trait RoleBook: Send + Sync {
    /// Alle sichtbaren Rollen für die Auswahl, nach Name.
    async fn roles(&self) -> Result<Vec<RoleInfo>>;
    /// Eine Rolle; `None`, wenn es sie nicht (mehr) gibt (Anzeige: der Verweis selbst).
    async fn role(&self, role: &RoleRef) -> Result<Option<RoleInfo>>;
    /// Wirksame Rollen einer Person (wie `Roles::effective_roles`), für die Anzeige in fs-people.
    async fn roles_of(&self, person: &PersonRef) -> Result<Vec<EffectiveRole>>;
}
