//! Spiegel der Rollen in Verzeichnisse (Kanidm): Rollen-Gruppen mit Präfix `role_` (Nutzerentscheidung
//! 2026-11-16, pilot-access §7a). fs-groups legt fest, Kanidm ist abgeleiteter Spiegel und schreibt nie zurück.
//! EIN Spiegel-Plan für Gruppen und Rollen (Strategie 16.11.: eine Freigabe, ein Ticket): `Mirroring` plant
//! Rollen-Schritte (`MirrorStep::Role`) im selben Plan; das Verzeichnis ist `GroupDirectory` mit den
//! Rollen-Methoden (`mirror.rs`). Diese Datei hält nur die Werte.
//!
//! Pattern: Value Object (Ziel, Eintrag, Schritt der Rollen-Spiegelung; Strategy/Adapter/Memento in `mirror.rs`).
//!
//! Abbildung (NATIV, nichts flach ausgerechnet, wie bei Gruppen):
//! - Je (Rolle, Geltung) mit mindestens einem Inhaber EINE Kanidm-Gruppe. Name LESBAR (Nutzer will Rollen in
//!   Kanidm sehen und selbst eingeben): `role_<key>` (ohne Geltung) bzw. `role_<key>_<gid8>` (gid8 = die ersten
//!   8 Hex-Zeichen der Gruppen-UUID, KLEIN). Kanidm-Regel `^[a-z][a-z0-9-_\.]{0,63}$`, darum `key` höchstens 50.
//!   Kollision (gleicher gid8, ein globaler Name gleich einem gruppenbezogenen, oder eine fremde Kanidm-Gruppe
//!   heißt so): ein Name, der schon im Ledger steht, BLEIBT; es weicht immer der NEUE aus. Eine neue
//!   gruppenbezogene Rolle nimmt den nächsten freien Anhang (gid9, gid10 …: so viele Hex-Zeichen der UUID ohne
//!   `-`, klein), solange der Name höchstens 64 Zeichen hat. Eine neue GLOBALE Rolle kann nicht ausweichen. Ohne
//!   freien Namen: Befund `RoleNameTaken`, die Rolle bleibt draußen (nie still). Ein Umbenennen in Kanidm ändert
//!   keinen Token (der Claim hängt nicht am Namen).
//! - Leitquelle bleibt das Spiegel-Ledger (UUID der Kanidm-Gruppe → `RoleTarget`); Namen werden nie geparst.
//!   fs-access liest Rollen-Gruppen aus dem Claim `groups` heraus über die UUID im Ledger.
//! - Anzeigename und Beschreibung der Rolle gehen nach Kanidm `displayname` bzw. `description`.
//! - `member`: Personen mit direkter Herkunft (Zuordnung, `Membership.role`) und Inhaber-Gruppen `fsg-<uuid>`
//!   (Hülle über Kanidm `memberof`); Rollen-Kanten wie Gruppen-Kanten: `BottomUp` = Kind-Rollengruppe in `member`
//!   der Eltern-Rollengruppe (gleiche Geltung), `TopDown` umgekehrt, `Single` nicht gespiegelt.
//! - Claim-Map je OAuth2-Client: jede Rollen-Gruppe nur mit IHREM Wert (`claim` = `<key>` bzw. `<key>:<uuid>`,
//!   `role_holders.rs`); die Vererbung kommt über `memberof` (fs-identity, pilot-access A8). Keine SCIM-Groups.
//! - Nie gespiegelt: Rollen mit `mirror = false`, sensibel oder KI (Befund `RoleExcluded`; steht eine aus
//!   Altdaten im Verzeichnis: `RoleStep::Remove`). Nicht gespiegelte Inhaber-Gruppen: `HolderNotMirrored`. Rechte
//!   und Verwaltung (`managers`) werden nicht gespiegelt (O11).
//! - KI-Kette und Unbekannte auf der Hülle wie `mirror.rs`: ein Bogen oder Mitglied, das eine KI oder eine Person
//!   mit unbekanntem Status wirksam in eine admin-wirksame Rolle brächte, wird nicht geschrieben (`LeftOut` +
//!   `HeldBack` mit der Rollen-Id); der Plan läuft weiter und plant `remove`, wo sie schon gespiegelt ist.
//! - Reihenfolge im Plan: Gruppen-Schritte vor Rollen-Schritten (`fsg-<uuid>` muss stehen, bevor eine
//!   Rollen-Gruppe sie aufnimmt). Von Hand gesetzte Bögen: `ForeignNesting` bzw. `Foreign` (Bestätigung je Fall).

use super::shared::{ObjectId, PersonRef};
use super::DirectoryAccount;

/// Eine Rolle in einer Geltung: das, was EINE Rollen-Gruppe im Verzeichnis abbildet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoleTarget {
    pub role: ObjectId,
    pub within: Option<ObjectId>,
}

/// Eine Rollen-Gruppe, wie sie im Verzeichnis stehen soll oder steht. `members` nur direkte Personen; `groups`
/// = Inhaber-Gruppen (`fsg-<uuid>`); `roles` = Rollen-Gruppen, die in Kanidm Mitglied dieser sind (Rollen-Kanten
/// nach Modus); `foreign` nur beim Lesen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleDirectoryEntry {
    pub target: RoleTarget,
    /// `role_<key>` bzw. `role_<key>_<gid8>` (bei Kollision längerer Anhang aus dem Ledger, siehe Kopf).
    pub name: String,
    /// Kanidm `displayname` (= `Role::name`).
    pub display_name: String,
    /// Kanidm `description` (= `Role::description`).
    pub description: Option<String>,
    /// Claim-Wert ohne Präfix (für die Claim-Map `roles`).
    pub claim: String,
    pub members: Vec<PersonRef>,
    pub groups: Vec<ObjectId>,
    pub roles: Vec<RoleTarget>,
    pub foreign: Vec<DirectoryAccount>,
}

/// Ein geplanter Schritt an einer Rollen-Gruppe (in `MirrorStep::Role`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RoleStep {
    Create(RoleTarget),
    Update {
        target: RoleTarget,
        add: Vec<PersonRef>,
        remove: Vec<PersonRef>,
        nest_groups: Vec<ObjectId>,
        unnest_groups: Vec<ObjectId>,
        nest_roles: Vec<RoleTarget>,
        unnest_roles: Vec<RoleTarget>,
    },
    Remove(RoleTarget),
}
