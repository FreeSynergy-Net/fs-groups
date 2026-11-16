//! Recht "Admins benennen" (Nutzerentscheidung 31, 2026-11-16; Opus-Delta-Check B1, Strategie 16.11.): wer
//! Mitglieder in geschützte Gruppen aufnehmen und admin-wirksame Rollen vergeben darf.
//!
//! Pattern: Specification (die Regel steht hier, jeder Anbieter prüft sie gleich; Konformitätstest) + Value Object
//! (Schlüssel der Systemrolle, Grundlage einer Aufnahme durch den Türsteher).
//!
//! FORM: eine SYSTEMROLLE mit `key = ADMIN_APPOINTER_KEY` (`admin_appointer`, Kanidm `role_admin_appointer`),
//! `scope = Instance`, `RoleKind::Human { admin: true, sensitive: false }`; der Anbieter legt sie bei der
//! Einrichtung an. Sie lässt sich nicht löschen und ihre Art nicht ändern (`ModeConflict(SystemRole)`).
//! Gewählt statt eines Kennzeichens an der Gruppe, weil (1) ein Recht Personen gehört, nicht Gruppen: die Rolle
//! geht an Personen ODER Gruppen, mit Geltung (`within = g`: nur für Gruppe g; `None`: überall) und Verlauf;
//! (2) sie ist eine Admin-Rolle, also gelten KI-Kette, Unbekannte, Verbergung, Sperre, Befristung und Zustimmung
//! ohne neue Regel; (3) Kanidm zeigt sie als `role_admin_appointer` (Spiegel nativ, Leitquelle fs-groups).
//!
//! GESCHÜTZTE GRUPPE: eine Gruppe, deren Mitglieder dadurch eine ADMIN-WIRKSAME Rolle bekommen (zugeordnet an
//! sie oder an eine Gruppe, in die sie über ihre Hülle wirksam fließt, oder in ihr über `Membership.role`
//! vergeben; `Protection::admin`). Nur admin-wirksame (Strategie 16.11., H9): eine SENSIBLE Rolle oder Gruppe
//! braucht das Recht nicht, wie die direkte Vergabe einer sensiblen Rolle (`assign_role`); selbstverwaltet
//! verlangt sie aber `Confirm` oder `Vote` und keine Gruppen-Links (sonst `ModeConflict(SelfManaged)`).
//!
//! REGEL: Jede Handlung, durch die eine Person eine admin-wirksame Rolle NEU bekommt oder LÄNGER behält, braucht
//! zusätzlich zum bisherigen Recht das wirksam gehaltene `admin_appointer` (nicht gesperrt, nicht abgelaufen) mit
//! passender GELTUNG, sonst `Error::AppointerRequired { target }`. Das sind (vollständig):
//! - Mitglieder: `add_member`, `invite`, Annehmen (unten), `set_role`, `admit_member` (Türsteher, unten);
//!   `set_membership_until` in einer geschützten Gruppe beim VERLÄNGERN und ENTFRISTEN (Kürzen nicht).
//! - Rollen: `assign_role` (Person oder Gruppe), `role_nest`, `change_role_nesting_mode`, `role_update(kind)`,
//!   `answer_role_request` (geprüft wird die zustimmende Person), `role_restore`.
//! - Gruppen-Kanten: `request_nesting`, `answer_nesting_request` (die zustimmende Person), `change_nesting_mode`
//!   (Verstärken, Richtungswechsel), wenn durch die Kante Mitglieder wirksam in eine geschützte Gruppe fließen
//!   (`BottomUp`: das Kind und alles, was in es fließt, unter eine geschützte Gruppe; `TopDown`: die Eltern über
//!   einer geschützten). Geprüft wird, wer die Kante wirksam setzt: bei `Linked` die fragende Person, bei
//!   `Requested` die zustimmende beim Zustimmen (die Anfrage selbst braucht das Recht nicht). `Single` nie.
//! - `restore` (Gruppe) und `role_restore`: bekäme dadurch jemand eine admin-wirksame Rolle zurück (Mitglieder,
//!   Zuordnungen, Kanten), braucht die wiederherstellende Person das Recht; sonst `AppointerRequired`, nichts
//!   zurückgeholt. Abgelaufene Mitgliedschaften bleiben abgelaufen (`limits.rs`).
//!
//! GELTUNG: Das Recht muss die Geltung JEDER admin-wirksamen Rolle abdecken, die eine Person dadurch bekommt,
//! nicht die Eintrittsgruppe. Geltung der erworbenen Rolle = `within` der Zuordnung bzw. die Gruppe von
//! `Membership.role` (Claim `<key>:<gid>`), sonst die Instanz (`None`, auch über die Hülle und Kind-Gruppen).
//! Das Recht mit `within = None` deckt alles; mit `within = g` nur Rollen mit Geltung genau g (nicht Nachkommen,
//! im Zweifel nein). Ein Inhaber für g macht also nie Admins für die ganze Instanz.
//!
//! ANNEHMEN: geprüft wird das Recht der EINLADENDEN Person (`Invitation::invited_by`) ZUM ZEITPUNKT DER ANNAHME
//! (die eingeladene hält es nie). Fehlt es: `AppointerRequired`, nichts geschrieben, die Einladung bleibt offen
//! (Widerruf durch die Verwaltung). Zusätzlich: wird eine Gruppe GESCHÜTZT (durch `assign_role`, `set_role`,
//! Kante, Moduswechsel, `role_nest`, `role_update`, Zustimmen), zieht der Anbieter im selben Schritt alle offenen
//! Einladungen dieser Gruppe und jeder dadurch geschützten Gruppe zurück (`InvitationState::Revoked`, Verlauf
//! `InvitationClosed`, Akteur = handelnde Person); neue Einladungen prüft `invite` mit dem Recht.
//!
//! SELBSTVERGABE (feste Regel, Strategie 16.11. Frage 2): bekommt die HANDELNDE Person (beim Annehmen: die
//! einladende; beim Zustimmen: die zustimmende) dadurch selbst eine admin-wirksame Rolle neu oder länger (auch
//! `admin_appointer` selbst, auch über eine Gruppe, in der sie wirksam Mitglied ist, auch Selbstaufnahme in eine
//! geschützte Gruppe), genügt ihr Recht nicht: `Error::SecondAppointerRequired { target }`. Den Schritt tut ein
//! ZWEITER wirksamer Inhaber mit passender Geltung (selbst oder als Zustimmender), oder der Türsteher nach der
//! Aufnahme-Regel (die beantragende Person stimmt nicht mit). Ausnahme: der Start durch den Installierenden.
//!
//! RANGFOLGE: `NotFound` (verborgen) vor dem bisherigen Rechtefehler (`Forbidden`/`RoleForbidden`) vor
//! `AppointerRequired` vor `SecondAppointerRequired` vor allen Prüfungen des Inhalts.
//!
//! ANFANG: das erste Zuordnen macht der Installierende (ADR 0056 Punkt 1; in Kanidm die `idm_admins`). Danach
//! benennen Inhaber weitere (sie brauchen dazu `Manage` an der Rolle UND das Recht, und nie für sich selbst).
//!
//! TÜRSTEHER (`admission_v1`) entscheidet nichts selbst, er setzt um: er nimmt IMMER über `Groups::admit_member`
//! auf (sein Dienstkonto ist Einstellung des Anbieters; andere Konten: `Forbidden`) und gibt die Grundlage mit
//! (`AdmissionBasis`: Id der Entscheidung für das Protokoll, H10; der Mensch, dessen Handlung die Grundlage ist,
//! wird `added_by`). Rechte wie `add_member` des Dienstkontos; bei einer geschützten Gruppe gilt statt
//! `admin_appointer` die Aufnahme-Regel des Ziels (`SelfManagement::admission`): nur `Confirm` oder `Vote` mit
//! `by_appointers = true` und ohne Gruppen-Links, sonst `AppointerRequired`.
//!
//! ABSTIMMEN UND BESTÄTIGEN in geschützten Zielen (Strategie 16.11. Frage 1; fs-admission setzt es um, fs-groups
//! liefert die Inhaber): nur WIRKSAME Inhaber von `admin_appointer` mit passender Geltung (nicht gesperrt, nicht
//! abgelaufen, keine KI); wer beantragt und wer aufgenommen werden soll, stimmt nicht mit. Wer abstimmen darf,
//! wird beim START der Abstimmung festgehalten (eingefrorene Basis). VORGABEN für `admission_v1` (Frage 3;
//! VORGABE, WARTET AUF NUTZERBESTÄTIGUNG): `Vote` mit Schwelle ≥ 2/3 der Stimmberechtigten, mindestens 2
//! Stimmen; stimmberechtigt erst 30 Tage nach Beginn des Haltens des Rechts; keine Links. Gibt es weniger als 2
//! Stimmberechtigte, geht nur `Confirm` durch den Installierenden.
//!
//! SELBSTVERWALTUNG: ein geschütztes Ziel (Gruppe oder admin-wirksame Rolle) darf selbstverwaltet sein, aber NUR
//! mit `Confirm`/`Vote` durch Inhaber von `admin_appointer`, NIE `Open`, NIE Gruppen-Links, und nur, wenn es für
//! sie mindestens einen wirksamen Inhaber gibt. Wird ein selbstverwaltetes Ziel geschützt, während das nicht
//! erfüllt ist: `ModeConflict(SelfManaged)`; das gilt auch für Gruppen-Kanten und Moduswechsel (jede Gruppe, die
//! durch `BottomUp`/`TopDown` geschützt würde), Zustimmen und `restore`. Sensible Ziele: siehe oben. KI-Ziele
//! sind nie selbstverwaltet.
//!
//! KI: hält die Rolle nie (Admin-Rolle), stimmt nie zu, wird nie in eine geschützte Gruppe aufgenommen
//! (KI-Konten nur in KI-Gruppen, KI-Gruppen haben keine Nicht-KI-Rollen; `AiNesting`/`AiRoleForbidden`).

use super::shared::{ObjectId, PersonRef};

/// Schlüssel der Systemrolle "Admins benennen" (Claim `admin_appointer` bzw. `admin_appointer:<gruppen-uuid>`).
pub const ADMIN_APPOINTER_KEY: &str = "admin_appointer";

/// Grundlage einer Aufnahme durch den Türsteher (`Groups::admit_member`, Kopf der Datei).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionBasis {
    /// Id der Entscheidung in fs-admission (`admission_v1`); steht im Protokoll (`HistoryKind::MemberAdded`).
    pub decision: ObjectId,
    /// Der Mensch, dessen Handlung die Grundlage ist (einladende oder bestätigende Person); wird `added_by`.
    pub basis: PersonRef,
}
