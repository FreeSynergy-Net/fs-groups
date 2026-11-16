//! Der Trait `Groups` im Ordner `groups_v1/`: Gruppen, Verschachtelung, Mitglieder, Einladungen, Rechte (Rollen:
//! Trait `Roles`, `role_api.rs`).
//!
//! Pattern: Repository + Port (Trait; alle Methoden Pflicht, kein Null Object).

use super::shared::{Date, ObjectId, Page, PersonRef, Place, Reference, Revision, Utc};
use super::{
    AdmissionBasis, Block, BlockTarget, Change, Decision, Deleted, EffectiveGroup, EffectiveMember,
    Grant, Group, GroupDraft, GroupPatch, GroupQuery, GroupSummary, HistoryEntry, Invitation,
    InvitationAnswer, InvitationOutcome, Membership, NestOutcome, Nesting, NestingEffect,
    NestingMode, NestingRequest, NestingSide, RequestAnswer, RequestSide, Result, Right, RoleRef,
};
use async_trait::async_trait;

/// Fähigkeitskennungen (Wortlaut für Manifest, Katalog `provides` und `groups.capnp` v1, ADR 0068). Ein Anbieter
/// bietet alle; was die fragende Person nicht darf, ist `Error::Forbidden` (außer: KI und
/// Admin-/sensible Gruppe, dann `NotFound`, `nesting.rs`). `group.members` = Trait
/// `GroupMembers` (Kalender und Tasks finden den Anbieter darüber), `mirror.read` = Trockenlauf, Stand und
/// Bericht von `Mirroring` (Gruppen und Rollen, ein Plan), `mirror.write` = `mirror_apply`; `role.*` =
/// Trait `Roles`, `role.book` = `RoleBook` (fs-people findet den Anbieter darüber).
/// `member.block` = Sperren (`block`, `unblock`, `blocks`, `limits.rs`).
pub const CAPABILITIES: [&str; 18] = [
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
    "role.read",
    "role.write",
    "role.nest",
    "role.assign",
    "role.book",
    "member.block",
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
    /// Die fragende Person bekommt `Manage` an der neuen Gruppe; ist sie eine KI: `AgentNotAdmin`, nichts
    /// geschrieben. `ai` setzt, wer `Manage` hat, also nie eine KI.
    async fn create(&self, draft: GroupDraft) -> Result<Group>;
    /// `ai: true` an einer Gruppe, die als Inhaberin eine Nicht-KI-Rolle hält oder in der eine Nicht-KI-Rolle
    /// über `Membership.role` vergeben ist: `AiRoleForbidden` (`role_holders.rs`). `sensitive` prüft auch die
    /// Rollen-Zuordnungen der Gruppe (`SensitiveFlow`) und die Selbstverwaltung
    /// (`ModeConflict(SelfManaged)`).
    async fn update(&self, id: ObjectId, expected: Revision, patch: GroupPatch) -> Result<Group>;
    /// In den Papierkorb, mit Mitgliedschaften, Einladungen, Rechten, Kanten, offenen Anfragen, Rollen-Zuordnungen
    /// (Inhaberin oder Geltung) und Rollen mit `RoleScope::Group` dieser Gruppe (Kinder bleiben, verlieren diese
    /// Eltern). Spiegel: beim nächsten Abgleich entfernt.
    async fn delete(&self, id: ObjectId, expected: Revision) -> Result<Deleted>;
    /// Bis `restorable_until`; holt alles zurück, was `delete` mitnahm. Eine Kante, die inzwischen einen Kreis
    /// schlösse (oder zu tief, oder sensibel unverträglich wäre, oder die KI-Regel verletzte), bleibt weg und
    /// steht als `NestingRemoved` im Protokoll; Mitgliedschaften, die die KI-Regel inzwischen verletzen (Person
    /// inzwischen als KI markiert, Gruppe inzwischen `ai`), kommen zurück und zählen nicht
    /// (`InForce::KindMismatch`), ebenso Rollen-Zuordnungen gegen die KI-Kette (`role_holders.rs`); offene
    /// Anfragen von `delete` sind `Withdrawn` und kommen nicht zurück; abgelaufene Mitgliedschaften bleiben
    /// abgelaufen. Bekäme jemand dadurch eine admin-wirksame Rolle zurück: Recht nötig (`appoint.rs`).
    async fn restore(&self, id: ObjectId) -> Result<Group>;

    // ---- Verschachtelung (group.nest; Regel und Modi: `nesting.rs`)
    /// Kante "Kind gehört zu Eltern" mit Modus. `BottomUp`: Mitglieder zählen nach oben, die Rechte der oberen
    /// Gruppe gelten für alle darunter. `TopDown`: Mitgliedschaft fließt nach unten, Rechte fließen nach oben
    /// (Chefs in `member` von Vizechefs: Chefs dürfen, was die Vizechefs dürfen, und sind auch deren wirksame
    /// Mitglieder). Die fragende Person braucht `Manage` an mindestens einer Seite; an beiden: `Linked`, sonst
    /// `Requested` an die andere Seite. Geprüft (auch beim Zustimmen erneut): `Cycle` (auch Selbstkante und
    /// umgekehrtes Paar), `TooDeep`, `ModeConflict` (sensibel, beide Richtungen), `AiNesting` (KI-Gruppe nur als
    /// `Single`-Blatt, nie Eltern, nie `BottomUp`/`TopDown`); `Single` ist nur Anzeige, nicht gespiegelt; schon
    /// da: `AlreadyThere` (Kante in gleichem oder anderem Modus, oder offene Anfrage; trägt die Id). Fließen
    /// dadurch Mitglieder wirksam in eine geschützte Gruppe: Recht "Admins benennen" für die Person, die die Kante
    /// wirksam setzt, und `ModeConflict(SelfManaged)` für jede dadurch geschützte Gruppe (`appoint.rs`).
    async fn request_nesting(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestOutcome>;
    /// Was die Kante (oder ein Moduswechsel auf `mode`) bewirken würde, ohne zu schreiben; gleiche Prüfungen.
    /// `Manage` an einer Seite nötig; eine KI hat nie `Manage`, bekommt also nie Zahlen (`NotFound`, wenn eine
    /// Seite eine Admin-/sensible Gruppe ist, `nesting.rs`; sonst `Forbidden`). Zahlen: `NestingEffect`.
    async fn nesting_effect(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestingEffect>;
    /// Bestätigte Kanten der Gruppe auf einer Seite (Baumansicht mit Modus je Kante).
    async fn nestings(&self, group: ObjectId, side: NestingSide) -> Result<Vec<Nesting>>;
    /// Anfragen, an denen die Gruppe beteiligt ist, älteste zuerst (`Manage` an der Gruppe nötig). NUR Kanten des
    /// Gruppenbaums: Zuordnungs-Anfragen (`NestingRequestKind::Assign`) und Rollen-Kanten laufen über
    /// `Roles::role_requests`; `answer_`/`withdraw_nesting_request` auf deren Ids: `NotFound`. Verfall:
    /// `RequestState::Expired`.
    async fn nesting_requests(
        &self,
        group: ObjectId,
        side: RequestSide,
    ) -> Result<Vec<NestingRequest>>;
    /// Nur wer die gefragte Seite verwaltet. Eine KI nie (nach der Sichtbarkeitsregel, `nesting.rs`):
    /// `Forbidden { right: Manage }`, weil `Manage` für sie
    /// nie gilt (ADR 0056 Punkt 6). Zustimmen setzt die Kante bzw. den Modus und prüft alles erneut; hat sich die
    /// Kante seit der Anfrage geändert (`ChangeMode.from` stimmt nicht mehr), folgt `Changed`. Das Recht "Admins
    /// benennen" prüft die zustimmende Person (`appoint.rs`).
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
    /// (auch `AiNesting`, Recht "Admins benennen" und `SelfManaged` wie dort). Gleicher Modus: `AlreadyThere`.
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
    /// `AddMembers` nötig; mit Rolle ZUSÄTZLICH `Manage` an der Rolle ("Rolle vergeben", wie `assign_role`;
    /// sonst `RoleForbidden`). Gleiche Person mit gleicher Rolle: `AlreadyThere`. Gruppe als Person: `WrongArt`.
    /// KI-Konto in einer Menschengruppe, Mensch in einer KI-Gruppe: `AiNesting { group, path: [group] }`;
    /// Person mit unbekanntem KI-Status in einer KI-Gruppe: `AiStatusUnknown` (in Menschengruppen erlaubt).
    /// Gilt auch für `invite` und Annehmen. Wird eine Person erst später als KI markiert, bleibt die
    /// Mitgliedschaft stehen und zählt nicht (`InForce::KindMismatch`, `nesting.rs`; auch die umgekehrte
    /// Neumarkierung in einer KI-Gruppe). Mit Rolle zusätzlich: Geltung (`OutOfScope`), KI-Kette
    /// (`AiRoleForbidden`, `role_holders.rs`), Selbstverwaltung (`ModeConflict(SelfManaged)`). `until`: letzter
    /// Tag (`limits.rs`), vor heute: `Invalid` (`Field::Until`, `InPast`). Gesperrte Person (`Block` an der Gruppe):
    /// erlaubt, zählt nicht (`InForce::Blocked`). Geschützte Gruppe oder admin-wirksame Rolle: zusätzlich das
    /// Recht "Admins benennen" mit passender Geltung, für sich selbst ein zweiter Inhaber (`appoint.rs`).
    async fn add_member(
        &self,
        group: ObjectId,
        person: &PersonRef,
        role: Option<&RoleRef>,
        until: Option<Date>,
    ) -> Result<Membership>;
    /// Rolle ändern (`Manage` an der Gruppe UND, für die neue Rolle, `Manage` an ihr, sonst `RoleForbidden`;
    /// Admin-Rolle für KI: `AgentNotAdmin`; sonst Prüfungen wie `add_member`).
    async fn set_role(
        &self,
        membership: ObjectId,
        expected: Revision,
        role: Option<&RoleRef>,
    ) -> Result<Membership>;
    /// `RemoveMembers` nötig, oder die Person entfernt sich selbst (Austritt). `LastManager` möglich.
    async fn remove_member(&self, membership: ObjectId, expected: Revision) -> Result<()>;
    /// Befristung ändern (`limits.rs`): verlängern oder unbefristet (`None`) braucht `AddMembers`, in einer
    /// geschützten Gruppe zusätzlich das Recht "Admins benennen" (`appoint.rs`); kürzen `RemoveMembers` oder die
    /// Person selbst; vor heute: `Invalid` (`InPast`).
    async fn set_membership_until(
        &self,
        membership: ObjectId,
        expected: Revision,
        until: Option<Date>,
    ) -> Result<Membership>;
    /// Vorwarnung, nur lesend: befristete Mitgliedschaften, die in höchstens `days` Tagen enden, frühestes Ende
    /// zuerst. `group = Some(g)`: `Read` an g; `None`: die eigenen der fragenden Person (sichtbare Gruppen).
    async fn expiring_memberships(
        &self,
        group: Option<ObjectId>,
        days: u32,
    ) -> Result<Vec<Membership>>;
    /// Gruppen, in denen `person` wirksam Mitglied ist, mit Herkunft (nur, was die fragende Person sehen darf;
    /// `Inherited` nach der Regel "verborgen").
    async fn groups_of(&self, person: &PersonRef) -> Result<Vec<EffectiveGroup>>;

    // ---- Einladungen (invite.write, invite.answer)
    /// Einladungen einer Gruppe, neueste zuerst (`Read` nötig).
    async fn invitations(&self, group: ObjectId) -> Result<Vec<Invitation>>;
    /// Offene Einladungen an die fragende Person, neueste zuerst.
    async fn my_invitations(&self) -> Result<Vec<Invitation>>;
    /// `AddMembers` nötig, mit Rolle zusätzlich `Manage` an der Rolle (geprüft beim Einladen; Annehmen prüft
    /// Geltung und KI-Kette erneut); `expires` in der Zukunft. Schon Mitglied mit dieser Rolle: `AlreadyThere`.
    /// Geschützte Gruppe: Recht "Admins benennen" wie `add_member` (`appoint.rs`).
    async fn invite(
        &self,
        group: ObjectId,
        invitee: &PersonRef,
        role: Option<&RoleRef>,
        expires: Option<Utc>,
    ) -> Result<Invitation>;
    /// Nur die eingeladene Person. Annehmen legt die Mitgliedschaft an (ein Schritt, atomar); geschützte Gruppe:
    /// geprüft wird das Recht der EINLADENDEN Person zum Zeitpunkt der Annahme (`appoint.rs`).
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
    /// Aufnahme durch den Türsteher (`admission_v1`; nur sein Dienstkonto, sonst `Forbidden`), mit Grundlage
    /// (`basis.decision` ins Protokoll, `basis.basis` wird `added_by`). Prüfungen wie `add_member`; geschützte
    /// Gruppe: statt des Rechts die Aufnahme-Regel des Ziels (`appoint.rs`), sonst `AppointerRequired`.
    async fn admit_member(
        &self,
        group: ObjectId,
        person: &PersonRef,
        role: Option<&RoleRef>,
        until: Option<Date>,
        basis: &AdmissionBasis,
    ) -> Result<Membership>;

    // ---- Rechte (right.read, right.write)
    /// Vergebene Rechte einer Gruppe (`Read` nötig).
    async fn grants(&self, group: ObjectId) -> Result<Vec<Grant>>;
    /// `Manage` nötig. Gleiches Recht an gleichen Inhaber: `AlreadyThere`. `Manage` an KI: `AgentNotAdmin`;
    /// an einer Gruppe mit Admin-Recht oder einer sensiblen Gruppe jedes Recht an eine KI: `AgentNotAdmin`.
    async fn grant(&self, group: ObjectId, holder: &PersonRef, right: Right) -> Result<Grant>;
    /// `Manage` nötig; `LastManager` möglich.
    async fn revoke_grant(&self, grant: ObjectId, expected: Revision) -> Result<()>;
    /// Auskunft für eine beliebige Person, mit Grund; im Zweifel nein. Schreibt nichts.
    async fn may_do(&self, group: ObjectId, who: &PersonRef, right: Right) -> Result<Decision>;
    /// Rechte der fragenden Person an der Gruppe (das Frontend zeigt nur, was geht).
    async fn my_rights(&self, group: ObjectId) -> Result<Vec<Right>>;

    // ---- Sperren (member.block; Regel und Unterschied zum Ausschluss: `limits.rs`)
    /// Person an einer Gruppe oder Rolle sperren: `Manage` am Ziel (sonst `Forbidden`/`RoleForbidden`, nie für
    /// eine KI); eine Gruppe als Person: `WrongArt`; schon gesperrt: `AlreadyThere`; letzte Person mit `Manage`:
    /// `LastManager`, ebenso gegen den Installierenden (nie sperrbar); `until` vor heute: `Invalid`. Wirkt
    /// sofort (`InForce::Blocked`, `Reason::Blocked`).
    async fn block(
        &self,
        target: BlockTarget,
        person: &PersonRef,
        note: Option<&str>,
        until: Option<Date>,
    ) -> Result<Block>;
    /// Sperre aufheben (`Manage` am Ziel; der Installierende immer, `limits.rs`).
    async fn unblock(&self, block: ObjectId, expected: Revision) -> Result<()>;
    /// Sperren an einem Ziel (`Manage` nötig; der Installierende immer), neueste zuerst.
    async fn blocks(&self, target: BlockTarget) -> Result<Vec<Block>>;

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
}
