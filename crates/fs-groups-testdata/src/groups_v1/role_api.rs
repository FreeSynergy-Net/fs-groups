//! Der Trait `Roles` im Ordner `groups_v1/` (Entscheidung 21): Rollen, Rollenbaum, Zuordnungen, wirksame Rollen.
//! Zweiter Trait im selben Ordner (Interface Segregation: wer nur Gruppen braucht, hängt nicht an Rollen); ein
//! Anbieter von `Groups` bietet immer auch `Roles` (gleiche Fähigkeitsliste `CAPABILITIES`, `api.rs`).
//!
//! Pattern: Repository + Port (Trait; alle Methoden Pflicht, kein Null Object).
//!
//! Regeln wie `Groups` (`mod.rs`): wer handelt, ist die fragende Person des Anbieters; Schreiben auf Bestehendes
//! gegen `expected` (`Changed`), ausgenommen Anlegen (`role_create`, `assign_role`, `role_nest`) und
//! `role_restore`; Löschen = Papierkorb (`restorable_until`), `role_restore` holt Zuordnungen, Kanten und offene
//! Anfragen zurück (Prüfungen wie `restore`). Rechte an einer Rolle: `Right::Read` (Inhaber sehen, über
//! `Visibility` hinaus) und `Right::Manage` (alles); `AddMembers`/`RemoveMembers` gibt es an Rollen nicht (in
//! `Manage` enthalten). `Manage` hat, wer wirksam Mitglied von `Role::managers` ist, und der Installierende; nie
//! eine KI, nie eine Person mit unbekanntem Status. Fehlt es: `Error::RoleForbidden`. Für eine fragende KI sind
//! admin-wirksame und sensible Rollen verborgen (`role_holders.rs`): `NotFound`, nie `RoleForbidden`. Gruppen-Ids
//! erscheinen nur, wenn lesbar (`Visible::more_hidden`, `role_holders.rs` Sichtbarkeit (2)).
//!
//! Anfragen: Rollen-Kanten und Zuordnungen an Gruppen laufen NUR über `role_requests`/`answer_role_request`/
//! `withdraw_role_request`; `Groups::nesting_requests` listet sie nicht, `Groups::answer_nesting_request` und
//! `withdraw_nesting_request` antworten auf ihre Ids `NotFound(Missing::NestingRequest)` (und umgekehrt).
//! Offene Anfragen verfallen (`RequestState::Expired`, Regel in `nesting.rs`).

use super::shared::{ObjectId, Page, PageCursor, PersonRef, Reference, Revision};
use super::{
    AssignOutcome, Decision, Deleted, EffectiveRole, EffectiveRoleHolder, HistoryEntry,
    NestOutcome, Nesting, NestingEffect, NestingMode, NestingRequest, NestingSide, Protection,
    RequestAnswer, RequestSide, Result, Right, Role, RoleAssignment, RoleDraft, RolePatch,
    RoleQuery, RoleSummary, Visible,
};
use async_trait::async_trait;

/// Rollen von `FreeSynergy` Groups. Fähigkeiten: `role.read`, `role.write`, `role.nest`, `role.assign`.
#[async_trait]
pub trait Roles: Send + Sync {
    // ---- Rollen (role.read, role.write)
    /// Seitenweise, nach Name sortiert (stabil); es fehlen verborgene Rollen (KI) und Rollen mit
    /// `scope = Group(g)`, wenn g nicht lesbar ist (wie unbekannte Ids).
    async fn role_list(
        &self,
        query: &RoleQuery,
        after: Option<&PageCursor>,
        size: usize,
    ) -> Result<Page<RoleSummary>>;
    /// Gelöschte oder verborgene Rolle (KI; unlesbare Geltungs-Gruppe): `None`.
    async fn role_get(&self, id: ObjectId) -> Result<Option<Role>>;
    /// Über den Verweis (`urn:uuid:…`, `RoleRef`).
    async fn role_get_by_reference(&self, reference: &Reference) -> Result<Option<Role>>;
    /// `scope = Instance`: der Installierende, oder wem er das übertragen hat (wirksame Mitglieder der
    /// Instanz-Verwaltergruppe, Einstellung des Anbieters wie der Installierende; sonst `Forbidden` an dieser
    /// Gruppe); `scope = Group(g)`: `Manage` an g (sonst `Forbidden`). Danach verwaltet `managers`. KI:
    /// `AgentNotAdmin`. `key` schon vergeben (Regel `Role::key`, auch zwischen Instanz und Gruppe):
    /// `AlreadyThere`. `managers` eine KI-Gruppe:
    /// `AgentNotAdmin`. `mirror` bei sensibel oder KI: `Invalid` (`SensitiveNotMirrored`, `AiNotMirrored`).
    async fn role_create(&self, draft: RoleDraft) -> Result<Role>;
    /// `Manage` nötig. `kind` prüft Kette, Sensibel-Fluss und Selbstverwaltung (`RolePatch`); admin-wirksam machen
    /// braucht das Recht "Admins benennen" (`AppointerRequired`); Systemrolle: `ModeConflict(SystemRole)`.
    async fn role_update(&self, id: ObjectId, expected: Revision, patch: RolePatch)
        -> Result<Role>;
    /// Systemrolle `admin_appointer`: `ModeConflict(SystemRole)`. In den Papierkorb, mit Zuordnungen, Kanten und
    /// offenen Anfragen (`Withdrawn`). `Membership.role` auf die
    /// Rolle bleibt stehen und zählt bis `role_restore` nicht. Spiegel: beim nächsten Abgleich entfernt.
    async fn role_delete(&self, id: ObjectId, expected: Revision) -> Result<Deleted>;
    /// Bis `restorable_until`. Kanten, die inzwischen gegen Kreis, Tiefe, Sensibel oder KI-Regel verstießen,
    /// bleiben weg (`NestingRemoved`); Zuordnungen gegen die KI-Kette kommen zurück und zählen nicht. Bekäme
    /// jemand dadurch eine admin-wirksame Rolle zurück: Recht "Admins benennen" nötig (`appoint.rs`).
    async fn role_restore(&self, id: ObjectId) -> Result<Role>;

    // ---- Rollenbaum (role.nest; Regel `nesting.rs`, Rollen-Ids, Kopf von `role.rs`)
    /// Kante "Kind-Rolle gehört zu Eltern-Rolle" mit Modus; `Manage` an einer Seite, an beiden: `Linked`, sonst
    /// `Requested`. Geprüft (auch beim Zustimmen): Recht "Admins benennen" (`appoint.rs`), `Cycle`, `TooDeep`,
    /// `ModeConflict` (sensibel, Geltung, Selbstverwaltung; Instanz-Kind unter `Group(g)`-Eltern:
    /// `ScopeMismatch`), `AiRoleForbidden` (KI-Rolle nur `Single`-Blatt; Kette zu admin/sensibel),
    /// `AiStatusUnknown` (ein direkter Inhaber mit unbekanntem Status würde admin-wirksam); schon da:
    /// `AlreadyThere`.
    async fn role_nest(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestOutcome>;
    /// Wirkung ohne Schreiben (`groups_affected` = Zahl lesbarer Rollen; nur Lesbares, `NestingEffect`); `Manage` an
    /// einer Seite, eine KI hat es nie (`NotFound` bei verborgener Seite, sonst `RoleForbidden`).
    async fn role_nesting_effect(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestingEffect>;
    async fn role_nestings(&self, role: ObjectId, side: NestingSide) -> Result<Vec<Nesting>>;
    /// Anfragen (Rollen-Kanten und Zuordnungen an Gruppen), älteste zuerst; `target` = Rolle (`Manage` an ihr)
    /// oder Gruppe (`Manage` an ihr; nur Zuordnungs-Anfragen).
    async fn role_requests(
        &self,
        target: ObjectId,
        side: RequestSide,
    ) -> Result<Vec<NestingRequest>>;
    /// Nur die Verwaltung der gefragten Seite; eine KI nie (`RoleForbidden` bzw. `Forbidden`, `right: Manage`).
    /// Zustimmen prüft alles erneut (auch `AppointerRequired` für die zustimmende Person) und setzt die Kante bzw.
    /// die Zuordnung; abgelaufen: `RequestClosed(Expired)`.
    async fn answer_role_request(
        &self,
        request: ObjectId,
        expected: Revision,
        answer: RequestAnswer,
    ) -> Result<NestingRequest>;
    /// Wer die Anfrage stellte oder die fragende Seite verwaltet (wie `withdraw_nesting_request`); nur `Pending`
    /// (sonst `RequestClosed`). Ergebnis im Zustand `Withdrawn`.
    async fn withdraw_role_request(
        &self,
        request: ObjectId,
        expected: Revision,
    ) -> Result<NestingRequest>;
    /// Wie `change_nesting_mode` (zu `Single` einseitig und sofort; sonst beide Seiten, gleiche Prüfungen wie
    /// `role_nest`, auch Recht "Admins benennen" und `SelfManaged`, `appoint.rs`).
    async fn change_role_nesting_mode(
        &self,
        nesting: ObjectId,
        expected: Revision,
        mode: NestingMode,
    ) -> Result<NestOutcome>;
    /// Kante lösen (`Manage` an einer Seite); offene Anfragen dieser Kante: `Withdrawn`.
    async fn role_unnest(&self, nesting: ObjectId, expected: Revision) -> Result<()>;
    /// Vorfahren bzw. Nachkommen im Rollenbaum (alle Modi), nur sichtbare Rollen.
    async fn role_ancestors(&self, role: ObjectId) -> Result<Vec<ObjectId>>;
    async fn role_descendants(&self, role: ObjectId) -> Result<Vec<ObjectId>>;

    // ---- Zuordnungen und Auflösung (role.assign, role.read)
    /// An eine Person: `Manage` an der Rolle (sonst `RoleForbidden`), sofort `Assigned`. An eine Gruppe: derselbe
    /// Ablauf wie beim Einhängen (Strategie 16.11., kein Sonderweg): `Manage` an einer Seite nötig, an beiden
    /// `Assigned`, sonst `Requested` an die andere Seite (`NestingRequestKind::Assign`, child = Gruppe, parent =
    /// Rolle; eine KI stimmt nie zu). Geprüft (auch beim Zustimmen): Geltung (`OutOfScope`), Art (`WrongArt`),
    /// Sensibel-Fluss (Gruppe und Rolle gleich sensibel), KI-Kette, Selbstverwaltung, KI-Rolle mit Geltung g an
    /// einer Gruppe mit wirksamem "KI erlaubt: nein" (`AiNotAllowed`, `ai_access.rs`); admin-wirksame Rolle oder
    /// Gruppe wird geschützt: `AppointerRequired` ohne das Recht mit passender Geltung, an sich selbst
    /// `SecondAppointerRequired` (`appoint.rs`); offene Einladungen dadurch geschützter Gruppen: `Revoked`;
    /// schon da: `AlreadyThere`.
    async fn assign_role(
        &self,
        role: ObjectId,
        holder: &PersonRef,
        within: Option<ObjectId>,
    ) -> Result<AssignOutcome>;
    /// `Manage` an der Rolle oder an der Inhaber-Gruppe, oder die Person selbst (Austritt geht immer).
    async fn unassign_role(&self, assignment: ObjectId, expected: Revision) -> Result<()>;
    /// Direkte Zuordnungen einer Rolle, seitenweise; `Visibility` der Rolle gilt; Einträge mit unlesbarer
    /// Inhaber- oder Geltungs-Gruppe fehlen (`more_hidden`).
    async fn role_holders(
        &self,
        role: ObjectId,
        after: Option<&PageCursor>,
        size: usize,
    ) -> Result<Visible<Page<RoleAssignment>>>;
    /// Wirksame Inhaber (Auflösung `role_holders.rs`, nur `InForce::Yes`), seitenweise; `Visibility` gilt;
    /// unlesbare Geltung: fehlt (`more_hidden`).
    async fn effective_role_holders(
        &self,
        role: ObjectId,
        after: Option<&PageCursor>,
        size: usize,
    ) -> Result<Visible<Page<EffectiveRoleHolder>>>;
    /// Direkte Zuordnungen an eine Person (ohne `Membership.role`, die steht in `members`); unlesbare Geltung:
    /// fehlt (`more_hidden`).
    async fn roles_of(&self, person: &PersonRef) -> Result<Visible<Vec<RoleAssignment>>>;
    /// Direkte Zuordnungen an eine Gruppe (`Read` an der Gruppe nötig); unlesbare Geltung: `more_hidden`.
    async fn roles_of_group(&self, group: ObjectId) -> Result<Visible<Vec<RoleAssignment>>>;
    /// Wirksame Rollen mit Herkunft (`direct`/`inherited`, Claim-Wert); nur, was die fragende Person sehen
    /// darf (unlesbare Geltung: `more_hidden`). Quelle für fs-access (verbunden) und den SCIM-Abgleich, die als
    /// Dienst mit Leserecht an allen Gruppen fragen.
    async fn effective_roles(&self, person: &PersonRef) -> Result<Visible<Vec<EffectiveRole>>>;
    /// Schutz einer Gruppe oder Rolle (für fs-admission vor dem Einschalten der Selbstverwaltung). Nur für ein
    /// lesbares Ziel: `Read` nötig (sonst `Forbidden` bzw. `RoleForbidden`). Verborgenes Ziel (KI-Regel, oder eine
    /// `Group(g)`-Rolle mit unlesbarer g): `NotFound` VOR jedem anderen Fehler (kein Orakel für admin/sensibel).
    async fn protection(&self, target: ObjectId) -> Result<Protection>;

    // ---- Rechte, Protokoll (role.read)
    /// Auskunft für eine beliebige Person (`Read` oder `Manage` an der Rolle), mit Grund; im Zweifel nein.
    async fn may_do_role(&self, role: ObjectId, who: &PersonRef, right: Right) -> Result<Decision>;
    async fn my_role_rights(&self, role: ObjectId) -> Result<Vec<Right>>;
    /// Protokoll einer Rolle, älteste zuerst (`Read` nötig); Änderungen stehen auch in `Groups::changes`.
    async fn role_history(
        &self,
        role: ObjectId,
        after: Option<&PageCursor>,
        size: usize,
    ) -> Result<Page<HistoryEntry>>;
}
