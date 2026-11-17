//! Der Trait `Groups` im Ordner `groups_v2/` (alle v1-Methoden unverändert in Name und Parametern): Gruppen, Verschachtelung, Mitglieder, Einladungen, Rechte.
//!
//! Pattern: Repository + Port (Trait; alle Methoden Pflicht, kein Null Object).

use super::shared::{ObjectId, Page, PersonRef, Place, Reference, Revision, Utc};
use super::{
    Change, Decision, Deleted, EffectiveGroup, EffectiveMember, Grant, Group, GroupDraft,
    GroupPatch, GroupQuery, GroupSummary, HistoryEntry, Intervention, Invitation, InvitationAnswer,
    InvitationOutcome, Keep, Membership, NestOutcome, Nesting, NestingEffect, NestingMode,
    NestingRequest, NestingSide, RequestAnswer, RequestSide, Result, Right, RoleRef, Scope,
    ScopedGrant, Transfer,
};
use async_trait::async_trait;

/// Fähigkeitskennungen (Wortlaut für Manifest, Katalog `provides` und `groups.capnp` v1, ADR 0068). Ein Anbieter
/// bietet alle; was die fragende Person nicht darf, ist `Error::Forbidden` (außer: KI und
/// Admin-/sensible Gruppe, dann `NotFound`, `nesting.rs`). `group.members` = Trait
/// `GroupMembers` (Kalender und Tasks finden den Anbieter darüber), `mirror.read` = Trockenlauf, Stand und
/// Bericht von `Mirroring`, `mirror.write` = `mirror_apply`, `member.own` = Eigentum (`owners`,
/// `transfer_ownership`), `group.intervene` = Bereiche und Eingriffe (`Scopes` auf dem Draht).
pub const CAPABILITIES: [&str; 14] = [
    "group.read",
    "group.write",
    "group.nest",
    "member.read",
    "member.write",
    "invite.write",
    "invite.answer",
    "right.read",
    "right.write",
    "group.members",
    "mirror.read",
    "mirror.write",
    "member.own",
    "group.intervene",
];

/// Die Schnittstelle von `FreeSynergy` Groups. Wer handelt, ist die fragende Person des Anbieters.
#[async_trait]
pub trait Groups: Send + Sync {
    // ---- Gruppen (group.read, group.write)
    /// Seitenweise, nach Name sortiert (Unicode-Sortierung des Anbieters, stabil). Gelöschte nur mit
    /// `include_deleted`, erkennbar an `restorable_until`. Für eine fragende KI fehlen Admin- und sensible Gruppen.
    async fn list(
        &self,
        query: &GroupQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<GroupSummary>>;
    /// Eine gelöschte Gruppe ergibt `None`; ebenso eine Admin- oder sensible Gruppe für eine fragende KI.
    async fn get(&self, id: ObjectId) -> Result<Option<Group>>;
    /// Über den Verweis (`urn:uuid:…`), z. B. aus einem `PersonRef { art: Group }` des Kalenders.
    async fn get_by_reference(&self, reference: &Reference) -> Result<Option<Group>>;
    /// Die fragende Person wird Eigentümerin (Mitgliedschaft mit `ROLE_OWNER`; v1: `Manage`); ist sie eine KI:
    /// `AgentNotAdmin`, ist sie betreut: `SupervisedCannotCreate` (b7 Pkt 9, "die verantwortliche Person kann
    /// die Gruppe anlegen"), unbekannt ob betreut: `SupervisedStatusUnknown`; nichts geschrieben. `ai` setzt,
    /// wer `Manage` hat, also nie eine KI.
    async fn create(&self, draft: GroupDraft) -> Result<Group>;
    async fn update(&self, id: ObjectId, expected: Revision, patch: GroupPatch) -> Result<Group>;
    /// Hat die Gruppe einen wirksamen Eigentümer, braucht es `Own` oder einen Eingriff von (a)/(b), sonst
    /// `Manage`. Kein `delete_by(actor)` auf dem Draht: der Akteur ist Eigenschaft des Anbieters; ein
    /// Systemlauf (Papierkorbfrist) handelt als Dienstkonto mit sichtbarem Protokolleintrag.
    /// In den Papierkorb, mit Mitgliedschaften, Einladungen, Rechten, Kanten und offenen Anfragen (Kinder
    /// bleiben, verlieren diese Eltern). Spiegel: beim nächsten Abgleich entfernt.
    async fn delete(&self, id: ObjectId, expected: Revision) -> Result<Deleted>;
    /// Bis `restorable_until`; holt alles zurück, was `delete` mitnahm. Eine Kante, die inzwischen einen Kreis
    /// schlösse (oder zu tief, oder sensibel unverträglich wäre, oder die KI-Regel verletzte), bleibt weg und
    /// steht als `NestingRemoved` im Protokoll; Mitgliedschaften, die die KI-Regel inzwischen verletzen (Person
    /// inzwischen als KI markiert, Gruppe inzwischen `ai`), kommen zurück und zählen nicht
    /// (`InForce::KindMismatch`); offene Anfragen von `delete` sind `Withdrawn` und kommen nicht zurück.
    async fn restore(&self, id: ObjectId) -> Result<Group>;

    // ---- Verschachtelung (group.nest; Regel und Modi: `nesting.rs`)
    /// Kante "Kind gehört zu Eltern" mit Modus. `BottomUp`: Mitglieder zählen nach oben, die Rechte der oberen
    /// Gruppe gelten für alle darunter. `TopDown`: Mitgliedschaft fließt nach unten, Rechte fließen nach oben
    /// (Chefs in `member` von Vizechefs: Chefs dürfen, was die Vizechefs dürfen, und sind auch deren wirksame
    /// Mitglieder). Die fragende Person braucht `Manage` an mindestens einer Seite; an beiden: `Linked`, sonst
    /// `Requested` an die andere Seite. Geprüft (auch beim Zustimmen erneut): `Cycle` (auch Selbstkante und
    /// umgekehrtes Paar), `TooDeep`, `ModeConflict` (sensibel, beide Richtungen), `AiNesting` (KI-Gruppe nur als
    /// `Single`-Blatt, nie Eltern, nie `BottomUp`/`TopDown`); `Single` ist nur Anzeige, nicht gespiegelt; schon
    /// da: `AlreadyThere` (Kante in gleichem oder anderem Modus, oder offene Anfrage; trägt die Id).
    async fn request_nesting(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestOutcome>;
    /// Was die Kante (oder ein Moduswechsel auf `mode`) bewirken würde, ohne zu schreiben; gleiche Prüfungen.
    /// `Manage` an einer Seite nötig, also für eine KI nie (`NotFound`, wenn eine Seite eine Admin-/sensible
    /// Gruppe ist, `nesting.rs`; sonst `Forbidden`); Zahlen nur über sichtbare Gruppen.
    async fn nesting_effect(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestingEffect>;
    /// Bestätigte Kanten der Gruppe auf einer Seite (Baumansicht mit Modus je Kante).
    async fn nestings(&self, group: ObjectId, side: NestingSide) -> Result<Vec<Nesting>>;
    /// Anfragen, an denen die Gruppe beteiligt ist, älteste zuerst (`Manage` an der Gruppe nötig).
    async fn nesting_requests(
        &self,
        group: ObjectId,
        side: RequestSide,
    ) -> Result<Vec<NestingRequest>>;
    /// Nur wer die gefragte Seite verwaltet. Eine KI nie (nach der Sichtbarkeitsregel, `nesting.rs`):
    /// `Forbidden { right: Manage }`, weil `Manage` für sie
    /// nie gilt (ADR 0056 Punkt 6). Zustimmen setzt die Kante bzw. den Modus und prüft alles erneut; hat sich die
    /// Kante seit der Anfrage geändert (`ChangeMode.from` stimmt nicht mehr), folgt `Changed`.
    async fn answer_nesting_request(
        &self,
        request: ObjectId,
        expected: Revision,
        answer: RequestAnswer,
    ) -> Result<NestingRequest>;
    /// Wer die Anfrage stellte oder die fragende Seite verwaltet.
    async fn withdraw_nesting_request(
        &self,
        request: ObjectId,
        expected: Revision,
    ) -> Result<NestingRequest>;
    /// Modus einer Kante ändern. Zu `Single` (Abschwächen) genügt `Manage` an einer Seite und gilt sofort;
    /// Verstärken oder Richtungswechsel braucht beide Seiten wie `request_nesting`, mit den gleichen Prüfungen
    /// (auch `AiNesting`). Gleicher Modus: `AlreadyThere`.
    async fn change_nesting_mode(
        &self,
        nesting: ObjectId,
        expected: Revision,
        mode: NestingMode,
    ) -> Result<NestOutcome>;
    /// Kante lösen: `Manage` an einer Seite genügt (Austritt geht immer). Offene Anfragen dieser Kante stehen
    /// danach auf `Withdrawn`.
    async fn remove_nesting(&self, nesting: ObjectId, expected: Revision) -> Result<()>;
    /// Alle Vorfahren (alle Modi), Breitensuche, jede Gruppe einmal; endet auch auf Fremddaten mit Schleife.
    /// Nur Gruppen, die die fragende Person lesen darf (sensible und verborgene fehlen, auch auf dem Weg; für
    /// eine KI auch Admin-Gruppen).
    async fn ancestors(&self, id: ObjectId) -> Result<Vec<ObjectId>>;
    /// Alle Nachkommen, wie `ancestors` (gleicher Sichtbarkeitsfilter).
    async fn descendants(&self, id: ObjectId) -> Result<Vec<ObjectId>>;

    // ---- Mitglieder (member.read, member.write)
    /// Direkte Mitglieder, seitenweise; `Visibility` gilt.
    async fn members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<Membership>>;
    /// Wirksame Mitglieder (Regel `nesting.rs`: direkt und über die Hülle des Mitgliedsgraphen), je Person einmal
    /// mit Herkunft "direkt"/"geerbt aus X", seitenweise in stabiler Reihenfolge des Anbieters; `Visibility` gilt.
    async fn effective_members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<EffectiveMember>>;
    /// `AddMembers` nötig; mit `ROLE_OWNER` `Own` (oder Eingriff) und ein menschliches, nicht betreutes Ziel
    /// (`SupervisedCannotOwn`/`SupervisedStatusUnknown`), mit `ROLE_MANAGER` `Manage`. `Some(ROLE_MEMBER)` wird
    /// zu `None`. Gleiche Person mit gleicher Rolle: `AlreadyThere`. Gruppe als Person: `WrongArt`.
    /// KI-Konto in einer Menschengruppe, Mensch in einer KI-Gruppe: `AiNesting { group, path: [group] }`;
    /// Person mit unbekanntem KI-Status in einer KI-Gruppe: `AiStatusUnknown` (in Menschengruppen erlaubt).
    /// Gilt auch für `invite` und Annehmen. Wird eine Person erst später als KI markiert, bleibt die
    /// Mitgliedschaft stehen und zählt nicht (`InForce::KindMismatch`, `nesting.rs`; auch die umgekehrte
    /// Neumarkierung in einer KI-Gruppe).
    async fn add_member(
        &self,
        group: ObjectId,
        person: &PersonRef,
        role: Option<&RoleRef>,
    ) -> Result<Membership>;
    /// Rolle ändern (`Manage` nötig; Admin-Rolle für KI: `AgentNotAdmin`; zum oder vom Eigentümer `Own`,
    /// Herabstufen der letzten Eigentümerin: `LastOwner { why: Demote }`).
    async fn set_role(
        &self,
        membership: ObjectId,
        expected: Revision,
        role: Option<&RoleRef>,
    ) -> Result<Membership>;
    /// `RemoveMembers` nötig, oder die Person entfernt sich selbst (Austritt); einen Eigentümer entfernen braucht
    /// `Own`. `LastOwner` und `LastManager` möglich, mit Grund (`Remove` oder `Leave`).
    async fn remove_member(&self, membership: ObjectId, expected: Revision) -> Result<()>;
    /// Gruppen, in denen `person` wirksam Mitglied ist, mit Herkunft (nur, was die fragende Person sehen darf;
    /// `Inherited` nach der Regel "verborgen").
    async fn groups_of(&self, person: &PersonRef) -> Result<Vec<EffectiveGroup>>;

    // ---- Einladungen (invite.write, invite.answer)
    /// Einladungen einer Gruppe, neueste zuerst (`Read` nötig).
    async fn invitations(&self, group: ObjectId) -> Result<Vec<Invitation>>;
    /// Offene Einladungen an die fragende Person, neueste zuerst.
    async fn my_invitations(&self) -> Result<Vec<Invitation>>;
    /// `AddMembers` nötig; `expires` in der Zukunft. Schon Mitglied mit dieser Rolle: `AlreadyThere`.
    async fn invite(
        &self,
        group: ObjectId,
        invitee: &PersonRef,
        role: Option<&RoleRef>,
        expires: Option<Utc>,
    ) -> Result<Invitation>;
    /// Nur die eingeladene Person. Annehmen legt die Mitgliedschaft an (ein Schritt, atomar).
    async fn answer_invitation(
        &self,
        invitation: ObjectId,
        expected: Revision,
        answer: InvitationAnswer,
    ) -> Result<InvitationOutcome>;
    /// `RemoveMembers` nötig, oder wer eingeladen hat.
    async fn revoke_invitation(
        &self,
        invitation: ObjectId,
        expected: Revision,
    ) -> Result<Invitation>;

    // ---- Rechte (right.read, right.write)
    /// Vergebene Rechte einer Gruppe (`Read` nötig).
    async fn grants(&self, group: ObjectId) -> Result<Vec<Grant>>;
    /// `Manage` nötig; `Right::Own` nie: `OwnViaRoleOnly`. Gleiches Recht an gleichen Inhaber: `AlreadyThere`. `Manage` an KI: `AgentNotAdmin`;
    /// an einer Gruppe mit Admin-Recht oder einer sensiblen Gruppe jedes Recht an eine KI: `AgentNotAdmin`.
    async fn grant(&self, group: ObjectId, holder: &PersonRef, right: Right) -> Result<Grant>;
    /// `Manage` nötig; `LastManager { why: RevokeGrant }` möglich.
    async fn revoke_grant(&self, grant: ObjectId, expected: Revision) -> Result<()>;
    /// Auskunft für eine beliebige Person, mit Grund; im Zweifel nein. Schreibt nichts.
    async fn may_do(&self, group: ObjectId, who: &PersonRef, right: Right) -> Result<Decision>;
    /// Rechte der fragenden Person an der Gruppe (das Frontend zeigt nur, was geht).
    async fn my_rights(&self, group: ObjectId) -> Result<Vec<Right>>;

    // ---- Beobachten und Protokoll (group.read)
    /// Änderungen seit `since` (`None` = ab jetzt), älteste zuerst; nur, was die fragende Person sehen darf.
    async fn changes(&self, since: Option<&Place>, size: usize) -> Result<Page<Change>>;
    /// Protokoll einer Gruppe, älteste zuerst (`Read` nötig).
    async fn history(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<HistoryEntry>>;

    // ---- Eigentum (member.own, ADR 0095)
    /// Eigentümer der Gruppe (direkte Mitgliedschaften mit `ROLE_OWNER`, auch nicht wirksame, erkennbar an
    /// `in_force`). IMMER sichtbar für jede Person, die die Gruppe lesen darf, AUCH bei `sensitive`
    /// (Antwort 101); nur `PersonRef`.
    async fn owners(&self, group: ObjectId) -> Result<Vec<Membership>>;
    /// Übergabe in einem Schritt. Fragende Person = Person von `from` mit wirksamer Eigentümer-Rolle, sonst
    /// `Forbidden { right: Own }`; `expected` = Revision von `from` (`Changed`). `to`: direkte Mitgliedschaft
    /// derselben Gruppe (`NotFound(Membership)`), Art Person/Profil (`WrongArt`), bekannt Mensch
    /// (`AgentNotAdmin`/`AiStatusUnknown`), nicht betreut (`SupervisedCannotOwn`/`SupervisedStatusUnknown`),
    /// nicht gelöscht, noch nicht Eigentümer (`AlreadyThere`). Schreibfolge: erst `to` wird Eigentümer, dann
    /// `from` wird `keep`; ein Abbruch dazwischen hinterlässt zwei Eigentümer, nie keinen. Protokoll:
    /// `OwnershipTransferred`. Eine LEERSTELLE (`ownership.rs`) füllt man mit `add_member`/`set_role` unter
    /// Eingriff, nicht hiermit.
    async fn transfer_ownership(
        &self,
        from: ObjectId,
        expected: Revision,
        to: ObjectId,
        keep: Keep,
    ) -> Result<Transfer>;

    // ---- Bereiche und Eingriffe (group.intervene, `scope.rs`)
    /// Eingriff öffnen: die fragende Person ist direktes, menschliches, wirksames Mitglied einer Gruppe mit
    /// `ScopedGrant` für `Project` oder `AllGroups` (sonst `Forbidden`); `via` = dieses `ScopedGrant`;
    /// `reason` Pflicht (`ReasonMissing`). Schreibt `InterventionOpened` in das Protokoll von `group`. Der
    /// Eingriff schließt nach der eingestellten Frist von selbst (Standard eine Stunde, `expires`).
    async fn open_intervention(
        &self,
        group: ObjectId,
        via: ObjectId,
        reason: &str,
    ) -> Result<Intervention>;
    /// Vorzeitig schließen (nur die Person, die ihn öffnete); schreibt `InterventionClosed`.
    async fn close_intervention(&self, id: ObjectId) -> Result<Intervention>;
    /// Alle Bereichsrechte; lesbar für jede angemeldete Person.
    async fn scoped_grants(&self) -> Result<Vec<ScopedGrant>>;
    /// Nur über `Project`; `holder` ist eine Gruppe, `Scope::Group(_)` ist `Invalid`.
    async fn grant_scope(&self, holder: ObjectId, scope: Scope) -> Result<ScopedGrant>;
    /// Nur über `Project`; der letzte `Project`: `LastScope`.
    async fn revoke_scope(&self, id: ObjectId, expected: Revision) -> Result<()>;
}
