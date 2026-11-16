//! Rechte an einer Gruppe (Tabelle `groups_grant`, PLAN.md §3), Auskunft `may_do`, Änderungsstrom, Protokoll.
//!
//! Pattern: Value Object (Recht, Entscheidung, Änderung) + Specification (die Regel liegt beim Anbieter, die
//! Antwort nennt ihren Grund).

use super::shared::{Date, ObjectId, PersonRef, Revision, Utc};
use super::{BlockTarget, InForce, InvitationState, NestingMode, RequestState, RoleField, RoleRef};

/// Ein Recht an einer Gruppe. `Manage` schließt die anderen ein. An einer Rolle gelten nur `Read` und `Manage`
/// (`role_api.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Right {
    /// Mitgliederliste, Einladungen, Rechte und Spiegel-Zustand lesen (über `Visibility` hinaus).
    Read,
    /// Mitglieder hinzufügen und einladen.
    AddMembers,
    /// Mitglieder entfernen und Einladungen widerrufen.
    RemoveMembers,
    /// Alles: Gruppe ändern, löschen, Kanten anfragen/beantworten/lösen, Rechte vergeben.
    Manage,
}

/// Vergebenes Recht. `holder.art`: `Person`, `Profile` oder `Group` (dann gilt es für die WIRKSAMEN Mitglieder
/// der Inhaber-Gruppe nach der Regel in `nesting.rs`, PLAN O1, O21). `BottomUp`: Mitglieder zählen nach oben,
/// die Rechte der oberen Gruppe gelten für alle darunter. `TopDown`: Mitgliedschaft fließt nach unten, Rechte
/// fließen nach oben: die obere Gruppe steht in `member` der unteren (Beispiel: 'Chefs' steht in `member` von
/// 'Vizechefs'; Chefs dürfen, was die Vizechefs dürfen, und sind auch wirksame Mitglieder der Vizechefs: sie
/// sehen deren Mitgliederliste und werden mit eingeladen). Eine KI-Gruppe als Inhaberin gilt nur für ihre
/// direkten Mitglieder (sie hat nie eine wirksame Kante) und nie für `Manage` (`Error::AgentNotAdmin`); an einer
/// Gruppe mit Admin-Recht oder einer sensiblen Gruppe hat eine KI gar keinen Zugriff (auch nicht `Read`,
/// `Error::AgentNotAdmin`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub id: ObjectId,
    pub revision: Revision,
    pub group: ObjectId,
    pub holder: PersonRef,
    pub right: Right,
    pub granted_by: PersonRef,
    pub granted: Utc,
    pub in_force: InForce,
}

/// Warum `may_do` bzw. `may_do_role` so antwortet. Im Zweifel nein.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reason {
    /// Recht direkt an der Person (`grant`), ggf. über `Manage`.
    Holder {
        grant: ObjectId,
    },
    /// Über eine Inhaber-Gruppe, in der die Person wirksam Mitglied ist (Hülle, `nesting.rs`). `BottomUp`:
    /// Mitglieder zählen nach oben, die Rechte der oberen Gruppe gelten für alle darunter. `TopDown`:
    /// Mitgliedschaft fließt nach unten, Rechte fließen nach oben (Chefs dürfen, was die Vizechefs dürfen).
    /// Rechte AN einer Gruppe gehen nicht über Kanten auf andere Gruppen über (im Zweifel nein).
    ViaGroup {
        grant: ObjectId,
        /// `None`, wenn die fragende Person die Inhaber-Gruppe nicht lesen darf (Regel "verborgen" wie
        /// `Inherited::from`: eine KI bei Admin-/sensiblen Gruppen, ein Mensch ohne Leserecht an einer sensiblen).
        holder: Option<ObjectId>,
    },
    /// Rolle: wirksames Mitglied der Verwaltergruppe `Role::managers`; `None`, wenn die fragende Person die
    /// Gruppe nicht lesen darf (Regel "verborgen").
    Manages {
        group: Option<ObjectId>,
    },
    /// Gesperrt (`limits.rs`): Sperre schlägt Erlaubnis, auch jeden Grant, jede Rolle und `Manage`; nie ja.
    /// Geprüft nach der Verbergung und nach `Installer` (nie gesperrt, B2), vor allen anderen Erlaubnissen.
    Blocked {
        block: ObjectId,
    },
    /// Wer installiert hat, ist oberster Admin (ADR 0056 Punkt 1), nicht auf einer Schleife; nie gesperrt (geht
    /// `Blocked` vor, `limits.rs`).
    Installer,
    NoGrant,
    /// Als KI markiert: nie `Manage` (ADR 0056 Punkt 6). Für eine KI zählen überhaupt nur `Holder` und `ViaGroup`
    /// einer KI-Gruppe ("Rechte nur ausdrücklich und einzeln"); ein Recht über eine Menschengruppe (auch eines,
    /// das ihr nach Neumarkierung noch anhängt, `InForce::KindMismatch`) ergibt `Agent`. An einer Gruppe mit
    /// Admin-Recht (Admin-Rolle vergeben) und an einer sensiblen Gruppe hat eine KI gar keinen Zugriff, auch nicht
    /// lesend; sie sieht die Gruppe nicht einmal (jede Methode: wie für eine nicht vorhandene Id, `nesting.rs`).
    /// An Rollen: eine KI verwaltet nie; admin-wirksame und sensible Rollen sieht sie nicht (`role_holders.rs`).
    Agent,
    /// Nicht bekannt, ob ein Mensch: nie `Manage`.
    AiStatusUnknown,
    /// Gruppe liegt auf einer Schleife (Fremddaten): nie ja.
    Loop,
    /// Ein Eintrag war nicht lesbar: nie ja.
    Unreadable,
}

/// Antwort von `may_do`. Fragt eine KI nach einer Gruppe, die sie nicht sehen darf (Admin-/sensibel), ist die Antwort
/// `NotFound` statt einer Entscheidung, auch wenn sie nach einer fremden Person fragt (`nesting.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub allowed: bool,
    pub reason: Reason,
}

/// Was sich geändert hat (für Beobachter: Fenster neu laden, Kalender/Tasks Mitglieder neu holen).
/// `Deleted`/`Restored` gelten für Gruppen und Rollen.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangeKind {
    Group(ObjectId),
    Membership {
        group: ObjectId,
    },
    Invitation {
        group: ObjectId,
    },
    /// Kante angelegt, Modus geändert oder gelöst (wirksame Mitglieder beider Seiten neu holen).
    Nesting {
        child: ObjectId,
        parent: ObjectId,
    },
    NestingRequest {
        child: ObjectId,
        parent: ObjectId,
    },
    Grant {
        group: ObjectId,
    },
    /// In den Papierkorb gelegt oder endgültig gelöscht.
    Deleted(ObjectId),
    Restored(ObjectId),
    Mirror {
        group: ObjectId,
    },
    Role(ObjectId),
    /// Zuordnung angelegt oder gelöst (wirksame Rollen der Inhaber neu holen).
    RoleAssignment {
        role: ObjectId,
    },
    /// Rollen-Kante angelegt, Modus geändert oder gelöst.
    RoleNesting {
        child: ObjectId,
        parent: ObjectId,
    },
    /// Anfrage im Rollenbaum oder Zuordnungs-Anfrage (dann `child` = Gruppe, `parent` = Rolle).
    RoleNestingRequest {
        child: ObjectId,
        parent: ObjectId,
    },
    /// Sperre gesetzt, aufgehoben oder abgelaufen (wirksame Mitglieder und Rollen neu holen).
    Block {
        target: BlockTarget,
    },
}

/// Ein Eintrag im Änderungsstrom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub at: Utc,
    pub kind: ChangeKind,
}

/// Geändertes Feld einer Gruppe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupField {
    Art,
    Name,
    Kind,
    Description,
    DefaultMode,
    Visibility,
    Sensitive,
    Ai,
    Mirror,
}

/// Typisiertes Protokoll einer Gruppe oder Rolle (ADR 0086 Punkt 2; Werte stehen in `activity_change`). Im
/// Protokoll einer Rolle nennen `Nesting*` mit `other` die Rolle am anderen Ende.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HistoryKind {
    Created,
    Updated {
        fields: Vec<GroupField>,
    },
    Deleted,
    Restored,
    /// Kante gesetzt; `other` = die Gruppe am anderen Ende (der Eintrag steht im Protokoll beider Gruppen).
    NestingLinked {
        other: ObjectId,
        mode: NestingMode,
    },
    NestingRequested {
        other: ObjectId,
        mode: NestingMode,
    },
    NestingAnswered {
        other: ObjectId,
        state: RequestState,
    },
    NestingModeChanged {
        other: ObjectId,
        from: NestingMode,
        to: NestingMode,
    },
    /// Gelöst; auch beim Wiederherstellen, wenn die Kante inzwischen einen Kreis schlösse.
    NestingRemoved {
        other: ObjectId,
    },
    /// `decision` = Id der Entscheidung des Türstehers (`Groups::admit_member`, `appoint.rs`), sonst `None`.
    MemberAdded {
        person: PersonRef,
        role: Option<RoleRef>,
        decision: Option<ObjectId>,
    },
    RoleChanged {
        person: PersonRef,
        from: Option<RoleRef>,
        to: Option<RoleRef>,
    },
    MemberRemoved {
        person: PersonRef,
    },
    Invited {
        invitee: PersonRef,
    },
    InvitationClosed {
        invitee: PersonRef,
        state: InvitationState,
    },
    Granted {
        holder: PersonRef,
        right: Right,
    },
    GrantRevoked {
        holder: PersonRef,
        right: Right,
    },
    RoleUpdated {
        fields: Vec<RoleField>,
    },
    /// Steht im Protokoll der Rolle, der Inhaber-Gruppe und der Geltungs-Gruppe.
    RoleAssigned {
        role: ObjectId,
        holder: PersonRef,
        within: Option<ObjectId>,
    },
    /// Zuordnungs-Anfrage gestellt (Antwort: `NestingAnswered` mit `other` = Gegenseite).
    RoleAssignRequested {
        role: ObjectId,
        group: ObjectId,
    },
    RoleUnassigned {
        role: ObjectId,
        holder: PersonRef,
        within: Option<ObjectId>,
    },
    /// Befristete Mitgliedschaft abgelaufen und entfallen (mit ihrer Rolle; `limits.rs`).
    MembershipExpired {
        person: PersonRef,
        role: Option<RoleRef>,
    },
    MembershipUntilChanged {
        person: PersonRef,
        from: Option<Date>,
        to: Option<Date>,
    },
    /// Im Protokoll des Ziels (Gruppe bzw. Rolle); nur für Inhaber von `Manage` sichtbar.
    Blocked {
        person: PersonRef,
    },
    /// Aufgehoben (`expired: false`) oder nach `Block::until` abgelaufen (`true`, `limits.rs`).
    Unblocked {
        person: PersonRef,
        expired: bool,
    },
}

/// Ein Protokolleintrag; `actor` = wer gehandelt hat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub at: Utc,
    pub actor: PersonRef,
    pub kind: HistoryKind,
}
