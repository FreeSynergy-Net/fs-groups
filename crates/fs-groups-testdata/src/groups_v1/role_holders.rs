//! Zuordnungen und wirksame Rollen (Tabelle `groups_role_assignment`, PLAN Entscheidung 21, pilot-access §7a).
//!
//! Pattern: Value Object (Zuordnung, Herkunft, wirksame Rolle) + Specification (Auflösung und KI-Kette stehen hier,
//! jeder Anbieter rechnet sie gleich; Konformitätstest; Rechnen im Crate `group-nesting`, zweite Instanz
//! `Tree<RoleId, PersonRef>`, dieser Ordner enthält keine Logik).
//!
//! Inhaber einer Rolle (Nutzerentscheidung 2026-11-16, Option C): eine Person (`Person`/`Profile`) oder eine
//! Gruppe (`Group`/`Organization`; dann alle WIRKSAMEN Mitglieder der Gruppe nach `nesting.rs`). Eine Zuordnung
//! ist KEINE Kante: sie geht nur Gruppe → Rolle, nie Rolle → Gruppe, darum läuft kein Kreis quer zwischen den
//! Bäumen; eine Rolle ist nie Mitglied einer Gruppe. Zusätzlich bleibt `Membership.role` (Person hat Rolle in
//! Gruppe G = wie eine Zuordnung an die Person mit `within = G`). Vergeben heißt auf JEDEM Weg dasselbe Recht:
//! `Manage` an der Rolle ("Rolle vergeben"; `assign_role`, `add_member`/`set_role`/`invite` mit Rolle; sonst
//! `RoleForbidden`), dazu das Recht des jeweiligen Weges an der Gruppe (Strategie 16.11., B4). Das gilt für die
//! Rolle in der Mitgliedschaft; wer eine Gruppe mit zugeordneter Rolle Mitglieder aufnehmen lässt, gibt ihnen die
//! Rolle (Option C, wie Kanidm); darum braucht Aufnehmen in solche Gruppen das Recht "Admins benennen"
//! (Entscheidung 31, `appoint.rs`).
//!
//! Auflösung `effective_roles(person)`: direkte Zuordnungen an die Person ∪ `Membership.role` ihrer Mitgliedschaften
//! ∪ Zuordnungen an jede Gruppe, in der sie wirksam Mitglied ist (`groups_of`), danach die Hülle im Rollenbaum
//! (Bögen nach Modus, `within` fließt mit). Jede (Rolle, Geltung) einmal, mit jeder Herkunft. Nur Einträge mit
//! `InForce::Yes` zählen; gelöschte Rollen, Gruppen und Zuordnungen zählen nicht.
//!
//! KI-Regel über die GANZE Kette (Nutzerentscheidung 2026-11-16; ADR 0054, 0056 Punkt 6). Kette = Inhaber →
//! (Gruppen-Hülle) → Zuordnung oder `Membership.role` → Rolle → (Rollen-Hülle) → jede erreichte Rolle.
//! - Ein KI-Konto und eine KI-Gruppe haben nur KI-Rollen; eine KI-Rolle hat nur KI-Inhaber (Mensch:
//!   `AiRoleForbidden`, unbekannter Status: `AiStatusUnknown`) und ist im Rollenbaum nur `Single`-Blatt (nie
//!   Eltern, keine wirksame Kante), erbt also nie und vererbt nie.
//! - Keine KI (Konto oder Gruppe) erreicht über irgendeine Kette eine Rolle mit `admin` oder `sensitive`; eine
//!   Person mit unbekanntem Status erreicht keine Rolle mit `admin` (zählt nicht, siehe unten).
//! - BEIM SCHREIBEN ist jeder Verstoß ein Fehler, nichts geschrieben, nie still: `AgentNotAdmin` (Admin-Rolle
//!   direkt an eine KI, wie bisher); `AiStatusUnknown`, wenn eine Person mit unbekanntem Status eine
//!   admin-wirksame Rolle DIREKT hätte (Zuordnung oder `Membership.role`), auch wenn die Rolle erst durch
//!   `role_nest`, Moduswechsel, Zustimmen oder `role_update(kind)` admin-wirksam würde; sonst `AiRoleForbidden
//!   { role, path }` (`path` = Inhaber-Gruppe falls vorhanden, dann die Rollen bis zur verbotenen). Geprüft bei
//!   `assign_role`, `add_member`/`set_role`/`invite`/Annehmen mit Rolle, `role_nest`, `change_role_nesting_mode`,
//!   Zustimmen, `role_update(kind)`, `role_restore`, `restore` (Gruppe) und `update(ai: true)` an einer Gruppe:
//!   hält sie als Inhaberin eine Nicht-KI-Rolle oder ist in ihr eine Nicht-KI-Rolle über `Membership.role`
//!   vergeben, ist das `AiRoleForbidden { role, path: [Gruppe, Rolle] }` (zusätzlich zu `AiNesting` für Menschen).
//! - Eine Person mit unbekanntem Status in einer Gruppe, der eine admin-wirksame Rolle zugeordnet ist: ERLAUBT
//!   (`add_member`, Gruppen-Kanten; wie in Gruppen mit Admin-Recht), zählt aber nicht: `effective_roles` liefert
//!   ihr die admin-wirksamen Rollen nicht, `effective_role_holders` nennt sie dort nicht, der Spiegel meldet
//!   `LeftOut` + `HeldBack` (Festlegung 16.11.).
//! - OHNE Schreiben falsch geworden (Neumarkierung, Altdaten, `restore`): die Zuordnung bleibt stehen, zählt
//!   nicht (`InForce::KindMismatch` bzw. `AiStatusUnknown`), Befund; im Spiegel `LeftOut` + `HeldBack`, `remove`.
//!
//! Sichtbarkeit: (1) Eine KI sieht keine admin-wirksame und keine sensible Rolle (jede Methode wie für eine
//! unbekannte Id, `NotFound` bzw. `None`, vor jedem anderen Fehler; ebenso Ids von Zuordnungen, Kanten und
//! Anfragen solcher Rollen; Einträge, die sie nennen, fehlen spurlos, ohne `more_hidden`). (2) Gruppen-Ids in
//! Rollen- und Zuordnungs-Auskünften (`RoleScope::Group`, `holder`, `within`, Claim) erscheinen nur, wenn die
//! fragende Person die Gruppe lesen darf (Strategie 16.11., B5): eine Rolle mit `scope = Group(g)` ist sonst
//! unsichtbar wie eine unbekannte Id, d. h. JEDE Methode antwortet `NotFound` bzw. `None`, VOR JEDEM ANDEREN
//! FEHLER (auch vor `RoleForbidden`, z. B. aus `protection`; sonst wäre es ein Orakel, H1). Einträge mit
//! unlesbarer Gruppe fehlen; wo die Antwort ein `Visible` ist, trägt sie dafür `more_hidden` ("weitere, nicht
//! sichtbar", ohne Zahl); Antworten ohne `Visible` (`role_nestings`, `role_requests`, `role_history`,
//! `RoleBook::roles_of`, `role_ancestors`/`role_descendants`) lassen Unlesbares still weg (H2). Für eine KI sind
//! die ihr verborgenen Gruppen Fall (1): spurlos.
//!
//! Befristung und Sperre (`limits.rs`): eine abgelaufene Mitgliedschaft entfällt samt `Membership.role` und den
//! Zuordnungen mit `within` = ihrer Gruppe; eine Sperre (Gruppe oder Rolle) nimmt der Person jede Rolle, die über
//! das gesperrte Ziel käme (Einträge `InForce::Blocked`). Abgelaufenes und Gesperrtes erscheint nie in
//! `effective_roles`/`effective_role_holders`.
//!
//! Claim und SCIM (pilot-access §7a, §14; Strategie 16.11., B1): Claim `roles` trägt `claim` OHNE Präfix: `<key>`
//! (ohne Geltung) bzw. `<key>:<uuid>` (UUID der Gruppe, Textform RFC 9562, klein, ohne `urn:uuid:`). Kanidm prüft
//! Claim-Werte gegen `^[0-9a-zA-Z_]+([:\-][0-9a-zA-Z_]+)*$`; die Form passt. SCIM `User.roles[].value` trägt
//! dieselbe Form; ein Eintrag je wirksamer Rolle und Art, `type` = `direct` und/oder `inherited` (`origins`).

use super::shared::{ObjectId, PersonRef, Revision, Utc};
use super::{InForce, Inherited, NestingRequest};

/// Direkte Zuordnung einer Rolle an einen Inhaber.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleAssignment {
    pub id: ObjectId,
    pub revision: Revision,
    pub role: ObjectId,
    /// `Person`/`Profile` oder `Group`/`Organization` (dann gilt sie für die wirksamen Mitglieder).
    pub holder: PersonRef,
    /// Geltung: `None` = ohne Gruppe (Claim `<key>`), `Some(g)` = in Gruppe g (Claim `<key>:<g>`).
    pub within: Option<ObjectId>,
    pub since: Utc,
    pub added_by: PersonRef,
    pub in_force: InForce,
}

/// Woher eine wirksame Rolle kommt. SCIM-`type`: `Assignment` und `Membership` = `direct`, sonst `inherited`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RoleOrigin {
    /// Zuordnung an die Person selbst (Id der Zuordnung).
    Assignment(ObjectId),
    /// `Membership.role` (Id der Mitgliedschaft; Geltung = deren Gruppe).
    Membership(ObjectId),
    /// Zuordnung an eine Gruppe, in der die Person wirksam Mitglied ist. `group = None`, wenn die fragende Person
    /// die Gruppe nicht lesen darf (Regel "verborgen" wie `Inherited::from`).
    ViaGroup { group: Option<ObjectId> },
    /// Über den Rollenbaum: `from` = Rolle, aus der geerbt wird, `via` = kürzester Weg aus Rollen-Ids (Regel
    /// "verborgen" wie bei Gruppen).
    ViaRole(Inherited),
}

/// Wirksame Rolle einer Person, je (Rolle, Geltung) einmal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveRole {
    pub role: ObjectId,
    pub within: Option<ObjectId>,
    /// Claim-Wert ohne Präfix (`<key>` bzw. `<key>:<uuid>`, siehe Kopf).
    pub claim: String,
    /// Mindestens eine Herkunft; kürzester Weg zuerst.
    pub origins: Vec<RoleOrigin>,
}

/// Wirksamer Inhaber einer Rolle (Personen, Art `Person`/`Profile`), mit Herkunft wie `EffectiveRole`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveRoleHolder {
    pub person: PersonRef,
    pub within: Option<ObjectId>,
    pub origins: Vec<RoleOrigin>,
}

/// Ergebnis von `assign_role`: sofort zugeordnet, oder Anfrage an die Seite, die die fragende Person nicht verwaltet.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssignOutcome {
    Assigned(RoleAssignment),
    Requested(NestingRequest),
}

/// Auskunft mit Hinweis auf Weggelassenes (Sichtbarkeit (2) im Kopf): `more_hidden = true` heißt "weitere,
/// nicht sichtbar", ohne Zahl. Nie gesetzt für Einträge, die einer KI verborgen sind (die fehlen spurlos).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visible<T> {
    pub value: T,
    pub more_hidden: bool,
}
