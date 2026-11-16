//! Befristung und Sperre (Nutzerantworten 26.1 und 26.2, 2026-11-16; Tabelle `groups_block`, Feld
//! `groups_membership.until`).
//!
//! Pattern: Value Object (Sperre, Ziel) + Specification (Wirkung von Ablauf und Sperre steht hier, jeder Anbieter
//! rechnet sie gleich; Konformitätstest).
//!
//! BEFRISTUNG (`Membership::until`, "Gast bis 31.12."): ein DATUM (`fs_types::Date`), kein Zeitpunkt, weil die
//! Person einen Tag meint und ein Datum ein Datum bleibt (Normalisierungsregel). Der Tag gilt EINSCHLIESSLICH; die
//! Mitgliedschaft endet um 00:00 des Folgetags in der Zeitzone des Anbieters (Einstellung der Instanz, wie die
//! Uhr; eine Zone je Gruppe gibt es in v1 nicht). Danach ENTFÄLLT sie: der Anbieter entfernt sie vor jedem Lesen
//! (wie abgelaufene Einladungen), Verlauf `HistoryKind::MembershipExpired` (mit der Rolle). Mit ihr entfallen
//! `Membership.role` und die Zuordnungen an die Person mit `within` = diese Gruppe (`RoleUnassigned` im Verlauf).
//! Endgültig (H8): `restore` der Gruppe holt eine inzwischen abgelaufene Mitgliedschaft NICHT zurück.
//! Abgelaufene Mitglieder zählen NIRGENDS: nicht in `effective_members`, `NestingEffect`, der Hülle über Kanten
//! (auch nicht nach oben über `BottomUp`), `may_do`, `effective_roles`, `GroupMembers`. Verlängern, Kürzen und
//! unbefristet machen: `Groups::set_membership_until` (in geschützten Gruppen braucht Verlängern und Entfristen
//! das Recht `admin_appointer`, `appoint.rs`); sofort entziehen: `remove_member`. Vorwarnung: nur die
//! Lesemethode `Groups::expiring_memberships` ("läuft ab in n Tagen"); Benachrichtigen ist Sache der Programme.
//! Einladung mit Ablaufdatum der Mitgliedschaft: [Platzhalter `admission_v1`; der Türsteher ruft `add_member`
//! mit `until`]. Spiegel: Leitquelle bleibt fs-groups; das Entfernen aus Kanidm ist ein Plan-Schritt
//! (`MirrorStep::Update.remove`, Befund `MembershipExpired`); in selbstverwalteten Gruppen entfernt der Türsteher
//! automatisch (`admission_v1`). ENTZUG OHNE VERZÖGERUNG (H7): ein `remove` wegen Ablauf oder Sperre wartet nicht
//! auf die Freigabe eines Plans (`mirror_apply`, Gate `human`): der Anbieter wendet genau diese Schritte selbst an,
//! sobald sie entstehen (eigener Lauf, `MirrorReport.actor` = sein Dienstkonto, im Bericht); alles andere bleibt
//! an `mirror_apply` gebunden. Bis dahin ist Kanidm im Modus "allein" veraltet; verbunden fragt fs-access frisch.
//! KI-Regeln unverändert (eine KI-Mitgliedschaft darf befristet sein).
//!
//! SPERRE (`Block`, "Sperre schlägt Erlaubnis", Nutzer Frage 15 JA) gegen AUSSCHLUSS:
//! - Ausschluss = `remove_member`: die Mitgliedschaft endet; über Kanten (Kind-Gruppen) oder Rollen kann die Person
//!   weiter wirken.
//! - Sperre = eine Person an einem Ziel (Gruppe oder Rolle) ohne jede Wirkung, egal woher sie kommt: Mitgliedschaft
//!   und Zuordnungen BLEIBEN stehen und sichtbar (`InForce::Blocked`), zählen aber nicht. Gruppe G gesperrt: die
//!   Person ist kein wirksames Mitglied von G (auch nicht über Kind-Gruppen) und nicht über G hinaus (nach oben),
//!   hat keine Rechte an G (`may_do` = nein, `Reason::Blocked`, auch für Grants und `Manage`) und keine Rolle, die
//!   über G kommt (Zuordnung an G, `Membership.role` in G, Geltung `within = G`). Rolle R gesperrt: die Person
//!   hält R nicht, aus keiner Herkunft, und nichts, was sie nur über R erreichte. Eine Sperre gilt auch ohne
//!   Mitgliedschaft (vorbeugend) und kann befristet sein (`until`, Datum wie oben). ENDE einer befristeten Sperre
//!   (H8): nach dem Tag `until` entfällt sie wie eine abgelaufene Mitgliedschaft (vor jedem Lesen entfernt, fehlt
//!   in `blocks`), Verlauf `HistoryKind::Unblocked { expired: true }`, Änderung `ChangeKind::Block`; die Einträge
//!   der Person zählen wieder (abgelaufene Mitgliedschaften bleiben abgelaufen).
//! - Grund: eine Sperre ist umkehrbar und nachvollziehbar (Verdacht, Klärung), ein Ausschluss endgültig; nur die
//!   Sperre wirkt gegen Rechte, die über Rollen oder Vererbung kommen.
//! - Wer: `Manage` am Ziel (Gruppe bzw. Rolle); nie eine KI (sie hat nie `Manage`); eine KI kann gesperrt werden.
//!   Die letzte Person mit `Manage` sperren: `LastManager`.
//! - DER INSTALLIERENDE IST NIE SPERRBAR (B2; ADR 0056 Punkt 1; Nutzerantwort 15 "Admins können die Sperre
//!   ändern"): `block` gegen ihn ist `LastManager`; `Reason::Installer` geht jeder Sperre vor (auch Sperren aus
//!   Altdaten wirken gegen ihn nicht). `blocks` und `unblock` stehen ihm an JEDEM Ziel offen, auch ohne `Manage`.
//!   Gesperrte Admins entsperrt, wer `Manage` am Ziel hat, und immer der Installierende.
//! - Auskunft und Audit: Notiz (`note`) und Verlauf (`Blocked`/`Unblocked`) sehen Inhaber von `Manage` und der
//!   Installierende; die gesperrte Person sieht ihre Sperre über `may_do` (`Reason::Blocked`) und `in_force`
//!   ihrer Einträge (Art. 15).
//! - Verborgen: Sperren an für die fragende Person verborgenen Zielen fehlen bzw. `NotFound` (Regeln `nesting.rs`,
//!   `role_holders.rs`).
//! - Spiegel: Leitquelle fs-groups. Direkte Mitgliedschaft und direkte Rollen der gesperrten Person werden in
//!   Kanidm entfernt (`Update.remove`). Kommt sie nur über einen Bogen (Kind-, Inhaber- oder Rollen-Gruppe)
//!   hinein, kann die Kanidm-Hülle das nicht ausdrücken (sie kennt keinen Ausschluss aus `memberof`; nicht live
//!   belegt): der Bogen bleibt (sonst träfe es alle), Befund `BlockNotMirrored`; verbunden fragt fs-access frisch.
//!   Ist das Ziel geschützt (admin-wirksam), schlägt der Befund vor, das Konto in Kanidm zu sperren
//!   (`account_expire`; der Mensch führt es aus, H7). Entfernen wegen Sperre: ohne Freigabe-Verzögerung (oben).

use super::shared::{Date, ObjectId, PersonRef, Revision, Utc};

/// Woran eine Person gesperrt ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockTarget {
    Group(ObjectId),
    Role(ObjectId),
}

/// Eine Sperre (Kopf der Datei).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub id: ObjectId,
    pub revision: Revision,
    pub target: BlockTarget,
    /// Art `Person`/`Profile` (eine Gruppe wird nicht gesperrt, sondern gelöst: `WrongArt`).
    pub person: PersonRef,
    /// Notiz für die Verwaltung (nur Inhaber von `Manage` sehen sie); kein Pflichtfeld.
    pub note: Option<String>,
    /// Letzter gesperrter Tag (einschließlich, Zone des Anbieters); `None` = bis zur Aufhebung.
    pub until: Option<Date>,
    pub since: Utc,
    pub blocked_by: PersonRef,
}
