//! Eigentümer einer Gruppe (ADR 0095): die drei mitgelieferten Rollen als feste IRIs, die Übergabe.
//!
//! Pattern: Value Object (feste Rolle, Übergabe-Ergebnis); die Schutzregel (Specification) steht bei `Error`.
//!
//! Namensschema: Basis von fs-people (`ROLE_BASE`) plus englisches Kurzwort, wie `ai` und `member`. Die drei
//! Rollen stehen FLACH im RoleBook (ohne `broader`, b7 Pkt 4); die Stufung tragen die Rechte: `ROLE_MANAGER`
//! trägt `Manage`, `ROLE_OWNER` trägt `Own` und damit `Manage`. Jede andere `RoleRef` gibt in v2 kein Recht.
//! Kanonische Form: "einfaches Mitglied" ist `role: None`; wer `Some(ROLE_MEMBER)` schreibt, bekommt beim
//! Schreiben `None` (Rückweg-Test K17). Mehrere Eigentümer sind ausdrücklich erlaubt (Antwort 99).
//!
//! LEERSTELLE (Entscheidung b7 Pkt 7 und 8): eine Gruppe hat keinen WIRKSAMEN Eigentümer, wenn (1) sie keinen
//! hat (Altdaten), (2) alle Eigentümer `InForce::KindMismatch`/`AiStatusUnknown` sind, (3) alle `InForce::Supervised`
//! sind (nachträglich betreut), oder (4) alle `InForce::Tombstone` sind: Eigentum einer GELÖSCHTEN Person (AS2
//! `Tombstone`) zählt als Leerstelle, der Eintrag bleibt als Spur stehen. Die Leerstelle füllen darf jede der
//! beiden oberen Stufen, (a) `Scope::Project` ODER (b) `Scope::AllGroups`, jeweils als Eingriff mit Grund und
//! Protokolleintrag (`scope.rs`); ein Verwalter allein nicht (`Forbidden { right: Own }`). Wer füllt, benennt einen
//! Menschen, der nicht betreut ist; es wird nie automatisch jemand Eigentümer.
//!
//! SPIEGEL (Entscheidung b7 Pkt 10): nach Kanidm schreibt NUR das Dienstkonto von fs-groups (`entry_managed_by`
//! ist dort das Dienstkonto, nie ein einzelner Eigentümer). Die Schutzregeln (`LastOwner`, `LastManager`,
//! KI-Sperre, Betreuungs-Sperre, Eingriff mit Grund) bleiben in fs-groups; Kanidm kennt sie nicht. Im Trait
//! steht davon nur die ANFORDERUNG: `Mirroring` (unverändert aus v1) bekommt keine Rechte der Eigentümer,
//! nur den fertigen Zustand.

use super::shared::BasicValue;
use super::{Membership, RoleRef};

/// Namensraum der mitgelieferten Rollen (gleich `ROLE_BASE` von fs-people).
pub const ROLE_MEMBER: &str = "https://freesynergy.net/ns/roles/member";
/// Verwalter: `Manage` (neu in `known.json` von fs-people, Besitzer fs-groups, `org:Role`, Geltung `group`).
pub const ROLE_MANAGER: &str = "https://freesynergy.net/ns/roles/manager";
/// Eigentümer: `Own` (und damit `Manage`); gleiche Ablage wie `ROLE_MANAGER`.
pub const ROLE_OWNER: &str = "https://freesynergy.net/ns/roles/owner";
/// Alt-Rollen (Texte der Migration, b7 Pkt 3): `ROLE_BASE` + `legacy/` + prozent-kodierter Text; kein Recht.
pub const ROLE_LEGACY_BASE: &str = "https://freesynergy.net/ns/roles/legacy/";

/// Die drei mitgelieferten Rollen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FixedRole {
    Member,
    Manager,
    Owner,
}

impl FixedRole {
    /// Die IRI der Rolle.
    #[must_use]
    pub const fn iri(self) -> &'static str {
        match self {
            Self::Member => ROLE_MEMBER,
            Self::Manager => ROLE_MANAGER,
            Self::Owner => ROLE_OWNER,
        }
    }

    /// Die feste Rolle zu einer Rolle der Mitgliedschaft: `None` ist einfaches Mitglied (`Member`); jede
    /// andere, unbekannte `RoleRef` (auch Alt-Rollen) ergibt `None`, also kein Recht.
    #[must_use]
    pub fn from_ref(role: Option<&RoleRef>) -> Option<Self> {
        let Some(role) = role else {
            return Some(Self::Member);
        };
        let iri = role.0.canonical();
        [Self::Member, Self::Manager, Self::Owner]
            .into_iter()
            .find(|r| iri == r.iri())
    }
}

/// Was die bisherige Eigentümerin nach der Übergabe ist (Eigentümer bleiben = Miteigentum, dafür `set_role`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keep {
    Manager,
    Member,
}

/// Ergebnis von `transfer_ownership`: beide Mitgliedschaften nach dem Schreiben.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub from: Membership,
    pub to: Membership,
}
