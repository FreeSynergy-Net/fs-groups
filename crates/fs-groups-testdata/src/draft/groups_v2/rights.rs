//! Rechte an einer Gruppe (Tabelle `groups_grant`, PLAN.md §3), Auskunft `may_do`, Änderungsstrom, Protokoll.
//!
//! Pattern: Value Object (Recht, Entscheidung, Änderung) + Specification (die Regel liegt beim Anbieter, die
//! Antwort nennt ihren Grund).

use super::shared::{ObjectId, PersonRef, Revision, Utc};
use super::{FixedRole, InForce, InvitationState, Keep, NestingMode, RequestState, RoleRef, Scope};

/// Ein Recht an einer Gruppe. `Manage` schließt die anderen ein.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Right {
    /// Mitgliederliste, Einladungen, Rechte und Spiegel-Zustand lesen (über `Visibility` hinaus).
    Read,
    /// Mitglieder hinzufügen und einladen.
    AddMembers,
    /// Mitglieder entfernen und Einladungen widerrufen.
    RemoveMembers,
    /// Alles: Gruppe ändern, löschen, Kanten anfragen/beantworten/lösen, Rechte vergeben. In einer Gruppe mit
    /// wirksamem Eigentümer NICHT: Eigentümer ernennen, herabstufen oder entfernen, Eigentum übergeben und die
    /// Gruppe löschen braucht `Own`.
    Manage,
    /// Eigentum (ADR 0095): schließt `Manage` ein. Hat NUR, wer die Rolle `ROLE_OWNER` in einer direkten,
    /// wirksamen (`InForce::Yes`) Mitgliedschaft hält; `grant(.., Own)` = `Error::OwnViaRoleOnly`. Fließt über
    /// keine Kante und keine Inhaber-Gruppe. Für eine KI oder Unbekannte: `allowed = false`.
    Own,
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

/// Warum `may_do` so antwortet. Im Zweifel nein.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// v1-Altlast, ein v2-Anbieter liefert sie nie: die einrichtende Person handelt als Eigentümerin von (a)
    /// über Eingriffe (ADR 0095 Pkt 6, `Reason::Intervention`).
    Installer,
    /// Recht aus der Verwalter- oder Eigentümer-Rolle einer direkten, wirksamen Mitgliedschaft.
    Role {
        membership: ObjectId,
        role: FixedRole,
    },
    /// Offener Eingriff von (a)/(b) in dieser Gruppe (`scope.rs`).
    Intervention {
        intervention: ObjectId,
        scope: Scope,
    },
    /// Betreutes Konto: nie `Own`.
    Supervised,
    NoGrant,
    /// Als KI markiert: nie `Manage` (ADR 0056 Punkt 6). Für eine KI zählen überhaupt nur `Holder` und `ViaGroup`
    /// einer KI-Gruppe ("Rechte nur ausdrücklich und einzeln"); ein Recht über eine Menschengruppe (auch eines,
    /// das ihr nach Neumarkierung noch anhängt, `InForce::KindMismatch`) ergibt `Agent`. An einer Gruppe mit
    /// Admin-Recht (Admin-Rolle vergeben) und an einer sensiblen Gruppe hat eine KI gar keinen Zugriff, auch nicht
    /// lesend; sie sieht die Gruppe nicht einmal (jede Methode: wie für eine nicht vorhandene Id, `nesting.rs`).
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Typisiertes Protokoll einer Gruppe (ADR 0086 Punkt 2; Werte stehen in `activity_change`).
#[derive(Clone, Debug, PartialEq, Eq)]
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
    MemberAdded {
        person: PersonRef,
        role: Option<RoleRef>,
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
    /// Eigentum übergeben (ein Eintrag, beide Personen; `keep` = was die bisherige Eigentümerin bleibt).
    OwnershipTransferred {
        from: PersonRef,
        to: PersonRef,
        keep: Keep,
    },
    /// Eingriff geöffnet: Admin-Gruppe (`holder`), Bereich und Grund; im Protokoll der betroffenen Gruppe.
    InterventionOpened {
        holder: ObjectId,
        scope: Scope,
        reason: String,
    },
    InterventionClosed,
    /// Bereich an eine Admin-Gruppe vergeben oder entzogen (`scope.rs`).
    ScopeGranted {
        holder: ObjectId,
        scope: Scope,
    },
    ScopeRevoked {
        holder: ObjectId,
        scope: Scope,
    },
}

/// Ein Protokolleintrag; `actor` = wer gehandelt hat. Einträge mit `intervention` liest JEDE Person, die die
/// Gruppe lesen darf (auch wenn sonst nur Verwalter das Protokoll sehen): Akteur, Bereich, Admin-Gruppe, Grund.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub at: Utc,
    pub actor: PersonRef,
    pub kind: HistoryKind,
    /// Der Eingriff, unter dem gehandelt wurde (`None` bei gewöhnlichen Handlungen).
    pub intervention: Option<ObjectId>,
}
