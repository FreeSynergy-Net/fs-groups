//! VERBRAUCHT, nicht unser Thema: was fs-groups von fs-people (ADR 0052, 0056, 0085) und fs-admission braucht.
//! Eigentümer: fs-people bzw. fs-admission. Stand-in der Form, die fs-groups braucht; die Brücken (Adapter, z. B.
//! `fs-groups-people`) setzen sie auf die echten Schnittstellen um. Weicht ein Eigentümer ab, ändert sich nur die
//! Brücke, nicht dieser Ordner, solange die Form hier erfüllbar bleibt.
//!
//! Pattern: Adapter (Personenverzeichnis, KI-Markierung und Selbstverwaltung an der Grenze, hinter Traits). Das
//! Rollenbuch (`RoleBook`) war hier verbraucht; seit Entscheidung 21 bietet fs-groups es an (`provided.rs`).

use super::shared::{ObjectId, PersonRef};
use super::Result;
use async_trait::async_trait;

/// Personen-Verweis auflösen. GLEICHE Form wie `appointments_v1::PersonDirectory` (gesperrt), damit fs-people
/// EINEN Anbieter für Kalender und Gruppen baut.
#[async_trait]
pub trait PersonDirectory: Send + Sync {
    /// Person zu einer Adresse (`mailto:…`), wenn bekannt.
    async fn find_by_address(&self, uri: &str) -> Result<Option<PersonRef>>;
    /// Unbekannte Adresse: Eintrag in der lokalen Ablage im Zustand "vorgeschlagen" (nie freier Name).
    async fn propose(&self, display_hint: &str, uri: &str) -> Result<PersonRef>;
    /// Anzeigename und Adresse für die Oberfläche (aktuell, nicht eingefroren).
    async fn display(&self, person: &PersonRef) -> Result<Option<(String, Option<String>)>>;
}

/// Ob eine Person eine KI ist (ADR 0054, 0076: Markierung in fs-people). `None` = unbekannt; dann zählt sie für
/// Admin-Regeln wie eine KI (im Zweifel nein). `Some(false)` nur, wenn bekannt ist, dass es ein Mensch ist.
/// Die Markierung setzt in fs-people nur ein Mensch, nie die markierte Person selbst, und jeder Wechsel löst
/// eine Neubewertung aus (Mitgliedschaften der KI zählen dann nicht mehr, `InForce::KindMismatch`). Grenze
/// (ADR 0054): eine Markierung schützt nur vor ehrlichen KIs; wer angreift, markiert sich nicht.
#[async_trait]
pub trait PersonFacts: Send + Sync {
    async fn is_agent(&self, person: &PersonRef) -> Result<Option<bool>>;
}

/// Art der Aufnahme-Regel (Stand-in; Eigentümer `admission_v1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdmissionRule {
    /// Gültige Einladung genügt.
    Open,
    /// Ein Mensch bestätigt.
    Confirm,
    /// Abstimmung.
    Vote,
}

/// Selbstverwaltung eines Ziels, so weit fs-groups sie prüft (`appoint.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    pub rule: AdmissionRule,
    /// Bestätigen bzw. abstimmen dürfen nur wirksame Inhaber von `admin_appointer` mit passender Geltung, ohne
    /// die beantragende Person; die Basis wird beim Start festgehalten (`appoint.rs`).
    pub by_appointers: bool,
    /// Gruppen-Links (QR, Link) erlaubt.
    pub links: bool,
}

/// Selbstverwaltung einer Gruppe oder Rolle. VERBRAUCHT von fs-admission (`admission_v1`, Eigentümer des
/// Kennzeichens und der Aufnahme-Regel; Entscheidung 21: Aufnahme ist ein eigener Baustein). fs-groups fragt es
/// vor jedem Schreiben, das ein Ziel geschützt oder KI machen würde (`ConflictKind::SelfManaged`), und wenn der
/// Türsteher in eine geschützte Gruppe aufnimmt (`appoint.rs`); fs-admission fragt umgekehrt `Roles::protection`,
/// bevor es einschaltet. Nicht erreichbar: `Unavailable(Unreachable::Admission)`, nichts geschrieben.
#[async_trait]
pub trait SelfManagement: Send + Sync {
    /// `None` = nicht selbstverwaltet.
    async fn admission(&self, target: ObjectId) -> Result<Option<Admission>>;
}
