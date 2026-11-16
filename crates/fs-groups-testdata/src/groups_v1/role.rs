//! Rolle und Abfrage (Tabelle `groups_role`; Rollenbaum in `groups_role_nesting`, Zuordnungen in
//! `role_holders.rs`). Entscheidung 21 (Nutzer, 2026-11-16): Rollen liegen IN `groups_v1`, kein eigener Trait-Ordner.
//!
//! Pattern: Value Object (Rolle, Entwurf, Änderung, Abfrage, Schutz).
//!
//! Leitquelle (sichtbar, Nutzerentscheidung 2026-11-16): fs-groups hält Rollen-Definitionen, Rollenbaum und
//! Zuordnungen; fs-people zeigt sie nur an (`RoleBook`, `provided.rs`); Kanidm ist abgeleiteter Spiegel
//! (`role_*`, `role_mirror.rs`) und schreibt nie zurück. Standard, wo es einen gibt: W3C `org:Role`, SKOS-Begriff
//! (`concept`), ADR 0052. Rollen als Beziehung einer Person (Kontakt, Kunde) bleiben bei fs-people.
//!
//! Rollenbaum: eigener Baum NUR aus Rollen (Kanten `Nesting`, Anfragen `NestingRequest`, Modi `NestingMode`,
//! Regel und Grenzfälle wie `nesting.rs`, gleiche Typen; in den Methoden von `Roles` sind `child`/`parent`
//! Rollen-Ids). Getrennt vom Gruppenbaum: es gibt keine Kante Gruppe ↔ Rolle. Kreise (`Cycle`), Tiefe
//! (`TooDeep`, `MAX_NESTING_DEPTH`), Sensibel (`ModeConflict(SensitiveFlow)`) und Zustimmung beider Seiten
//! (Verwaltung = `Role::managers`) gelten wie bei Gruppen. `BottomUp` (Vorgabe, wie Kanidm nativ): Inhaber der
//! unteren Rolle sind wirksam Inhaber der oberen (`editor` unter `member`: Editoren sind auch Mitglieder);
//! `TopDown` umgekehrt; `Single` nur Anzeige. Geltung im Rollenbaum (Festlegung 16.11.): eine Rolle mit
//! `scope = Group(g)` nimmt nur Kinder mit derselben Geltung `Group(g)` auf; ein INSTANZ-Kind (oder eines einer
//! anderen Gruppe) unter einem `Group(g)`-Elternteil ist verboten (`ModeConflict(ScopeMismatch)`, in jedem
//! Modus, auch `Single`). Ein `Group(g)`-Kind unter einer Instanz-Rolle ist erlaubt; die Geltung `within` einer
//! Zuordnung fließt mit (Inhaber von `x` in g sind wirksam Inhaber der Eltern-Rolle in g).

use super::shared::{ObjectId, PersonRef, Reference, Revision, Utc};
use super::{NestingMode, ParentFilter, Visibility};

/// Wo eine Rolle vergeben werden kann. Unveränderlich nach dem Anlegen (Claim-Werte bleiben stabil).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoleScope {
    /// Überall: ohne Geltung (Claim `<key>`) oder in jeder Gruppe (Claim `<key>:<gruppen-uuid>`).
    Instance,
    /// Nur in dieser Gruppe (Zuordnung mit `within = Some(g)`, `Membership.role` dieser Gruppe); sonst
    /// `Error::Invalid` (`Field::Within` bzw. `Field::Role`, `ProblemKind::OutOfScope`).
    Group(ObjectId),
}

/// Art einer Rolle. KI schließt Admin und sensibel aus (als Typ, nicht als Prüfung).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoleKind {
    /// Rolle für Menschen (und Personen mit unbekanntem Status, außer `admin`).
    Human {
        /// Admin-Rolle: trägt Admin-Rechte (fs-access bindet ein Admin-Recht nur an eine Rolle mit `admin`;
        /// Konformitätsregel). Nie für KI oder Personen mit unbekanntem Status, auch nicht über die Kette.
        admin: bool,
        /// Besondere Kategorie (DSGVO Art. 9): Inhaber sensibel, nie gespiegelt, nie exportiert; nur Gruppen mit
        /// gleichem `sensitive` als Inhaberinnen (`ModeConflict(SensitiveFlow)`).
        sensitive: bool,
    },
    /// KI-Rolle: nur KI-Inhaber (KI-Konten, KI-Gruppen); im Rollenbaum nur `Single`-Blatt
    /// (`Error::AiRoleForbidden`); nie Admin, nie sensibel.
    Ai,
}

/// Eine Rolle. Eltern und Kinder: `Roles::role_nestings`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Role {
    pub id: ObjectId,
    /// `urn:uuid:<id>`; Ziel jedes `RoleRef` (z. B. `Membership.role`).
    pub reference: Reference,
    pub revision: Revision,
    /// Claim-Wert OHNE Präfix (`editor`) und lesbarer Teil des Kanidm-Namens (`role_<key>` bzw.
    /// `role_<key>_<gid8>`, `role_mirror.rs`). Nur `a-z`, `0-9`, `_`, 1 bis 50 Zeichen (Kanidm-Namen höchstens 64
    /// Zeichen: `role_` + 50 + `_` + 8; Strategie 16.11.); sonst `Error::Invalid` BEIM ANLEGEN (`Field::Key`:
    /// `Empty`, `TooLong { max: 50 }`, `BadCharacters`), nie erst beim Spiegeln. Unveränderlich. Eindeutig über
    /// Instanz UND Gruppen (B3): ein Instanz-`key` gleicht keinem `key` einer `Group(_)`-Rolle und umgekehrt,
    /// zwei `Group(g)`-Rollen derselben Gruppe nie (sonst `Error::AlreadyThere(id der bestehenden Rolle)`).
    pub key: String,
    /// Anzeigename, nicht leer (Übersetzung über `concept`/ADR 0057 macht das Frontend).
    pub name: String,
    pub description: Option<String>,
    /// SKOS-Begriff oder `org:Role`-IRI, wo es einen Standard gibt (ADR 0052 Punkt 2).
    pub concept: Option<Reference>,
    pub scope: RoleScope,
    /// Gruppe, deren WIRKSAME Mitglieder die Rolle verwalten (`Right::Manage` an der Rolle; Form wie Kanidm
    /// `entry_managed_by`). Nie eine KI-Gruppe (`Error::AgentNotAdmin`); der Installierende verwaltet immer.
    pub managers: ObjectId,
    /// Modus vorgewählt für neue Kinder; Vorgabe beim Anlegen `BottomUp` (Nutzerentscheidung 2026-11-16).
    pub default_mode: NestingMode,
    /// Wer die Inhaber sehen darf (`Members` = wirksame Inhaber, `Managers` = Verwaltung).
    pub visibility: Visibility,
    pub kind: RoleKind,
    /// In Verzeichnisse spiegeln (`role_*`)? Bei `sensitive` und `RoleKind::Ai` immer `false`
    /// (`SensitiveNotMirrored`, `AiNotMirrored`).
    pub mirror: bool,
    pub created: Utc,
    pub created_by: PersonRef,
    pub updated: Utc,
}

/// Zeile einer Liste. Zahlen nur über SICHTBARE Rollen bzw. `None`, wo die Inhaber nicht sichtbar sind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleSummary {
    pub id: ObjectId,
    pub reference: Reference,
    pub revision: Revision,
    pub key: String,
    pub name: String,
    pub scope: RoleScope,
    pub default_mode: NestingMode,
    pub parents: u32,
    pub children: u32,
    pub direct_holders: Option<u32>,
    pub kind: RoleKind,
    pub mirror: bool,
    pub restorable_until: Option<Utc>,
}

/// Abfrage für `Roles::role_list`. `parent` wie bei Gruppen, mit Rollen-Ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleQuery {
    /// Teil von Name oder Schlüssel, ohne Groß/klein; leer = alle.
    pub text: String,
    /// `None` = jede Geltung.
    pub scope: Option<RoleScope>,
    pub parent: ParentFilter,
    /// Nur Rollen mit einer DIREKTEN Zuordnung an diese Person oder Gruppe.
    pub holder: Option<PersonRef>,
    pub include_deleted: bool,
}

/// Neue Rolle. Leere oder falsche Felder: `Error::Invalid`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleDraft {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub concept: Option<Reference>,
    pub scope: RoleScope,
    pub managers: ObjectId,
    pub default_mode: NestingMode,
    pub visibility: Visibility,
    pub kind: RoleKind,
    pub mirror: bool,
}

/// Änderung; `None` = Feld bleibt, `Some(None)` löscht ein freiwilliges Feld. `key` und `scope` sind
/// unveränderlich. `kind` prüft die KI-Kette und den Sensibel-Fluss über alle Inhaber und Kanten
/// (`AiRoleForbidden`, `ModeConflict`) und die Selbstverwaltung (`ModeConflict(SelfManaged)`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RolePatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub concept: Option<Option<Reference>>,
    pub managers: Option<ObjectId>,
    pub default_mode: Option<NestingMode>,
    pub visibility: Option<Visibility>,
    pub kind: Option<RoleKind>,
    pub mirror: Option<bool>,
}

/// Geändertes Feld einer Rolle (Protokoll `HistoryKind::RoleUpdated`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoleField {
    Name,
    Description,
    Concept,
    Managers,
    DefaultMode,
    Visibility,
    Kind,
    Mirror,
}

/// Schutz einer Gruppe oder Rolle (`Roles::protection`), für fs-admission: KI-Ziele sind nie selbstverwaltet,
/// geschützte nur mit passender Aufnahme-Regel (Pflichtprüfung in beiden Schichten, Entscheidung 31,
/// `appoint.rs`). `admin`/`sensitive` für eine Gruppe = ihre Mitglieder bekommen eine admin-wirksame bzw.
/// sensible Rolle (zugeordnet an sie oder an eine Gruppe, in die sie über ihre Hülle wirksam fließt, oder in ihr
/// über `Membership.role` vergeben; "Gruppe mit Admin-Recht"); für eine Rolle = sie selbst oder eine Rolle, die sie
/// über Bögen erreicht ("admin-wirksam"); `ai` = KI-Gruppe bzw. `RoleKind::Ai`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Protection {
    pub admin: bool,
    pub sensitive: bool,
    pub ai: bool,
}
