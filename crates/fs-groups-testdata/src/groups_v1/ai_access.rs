//! Gruppenfeld "KI erlaubt" (`Group::ai_allowed`; ADR 0054 Nachtrag 2, Nutzerentscheidung 39, 2026-11-16; Opus
//! `review-adr-0054-nachtrag2-opus.md`, `review-delta-rollen-5-opus.md` B4, H14–H19; Tabelle `groups_group`,
//! Spalte `ai_allowed INTEGER NOT NULL DEFAULT 0`).
//!
//! Pattern: Specification (Bedeutung, wirksamer Wert, Wirkung und Rangfolge stehen hier, jeder Anbieter rechnet
//! sie gleich; Konformitätstest; dieser Ordner enthält keine Logik) + Value Object (Voreinstellung).
//!
//! BEDEUTUNG: "KI erlaubt" ist NIE Mitgliedschaft. ADR 0082 (KI-Regel, `nesting.rs`) gilt weiter: ein KI-Konto
//! ist nie Mitglied einer Menschengruppe (`AiNesting`), auch nicht bei "ja". Das Feld umfasst genau
//! (i) KI-Teilnahme oder -Einladung in einer Unterhaltung, einem Raum oder einer Sitzung DER GRUPPE (0054
//! Nachtrag 1: Einladung = Teilnahme; die Programme mit Räumen lesen das Feld, Plan-Register, nicht hier) und
//! (ii) ein Recht einer KI AN dieser Gruppe: `Grant` einer KI-Gruppe oder eines KI-Kontos (`rights.rs`) und die
//! Zuordnung einer KI-Rolle mit Geltung g (`within = g` oder `RoleScope::Group(g)`, `role_holders.rs`; der Claim
//! `<key>:<g>` öffnet Programmen den Zugang zu g). "ja" ist nur eine Möglichkeit, keine Einladung und kein
//! Recht: es gelten weiter Einladung, Zustimmung aller Anwesenden, Vorwarnung Spätkommender und STOP KI (0054
//! Nachtrag 1). Der Programm-Schalter "KI-Zugang (MCP)" geht vor (Programm AUS = nie KI); er liegt nicht hier.
//!
//! GESPEICHERT UND WIRKSAM (B4): gespeichert ist die Spalte (Voreinstellung `AI_ALLOWED_DEFAULT` = `false`,
//! Anlegen-Dialog "KI erlaubt: nein"; Altdaten nach der Migration: `DEFAULT 0` = "nein"). WIRKSAM "ja" gilt nur,
//! wenn gespeichert "ja" UND die Gruppe weder sensibel (`Group::sensitive`) noch geschützt ist (`Protection`
//! der Gruppe, `role.rs`: weder `admin` noch `sensitive`, Hülle eingeschlossen) UND keine KI-Gruppe ist; sonst
//! wirksam "nein". Der wirksame Wert wird ABGELEITET, nicht aus einer Liste von Ereignissen: eine Gruppe, die
//! über irgendeinen Weg admin-wirksam wird (auch ein `BottomUp`-Kind unter einer Admin-Gruppe, ein `TopDown`-
//! Elternteil über einer), ist sofort wirksam "nein", ohne dass jemand an ihr handelt. JEDE Prüfung und Auskunft
//! nimmt den wirksamen Wert: `Group::ai_allowed`, `GroupSummary::ai_allowed`, `grant`, `assign_role`, `may_do`,
//! `GroupMembers` und die Räume der Programme. `ai = true` mit `ai_allowed = true` ist `Error::Invalid`
//! (`Field::AiAllowed`), bei `create` wie bei `update`.
//!
//! SETZEN (`GroupDraft::ai_allowed`, `GroupPatch::ai_allowed`): nur ein MENSCH mit `Manage` an der Gruppe. Eine
//! KI hat nie `Manage` und bekommt dieselbe Antwort wie für jedes andere `Manage`-Feld (`Forbidden`, an einer
//! für sie verborgenen Gruppe `NotFound`); hielte sie es aus Altdaten: `AgentNotAdmin`. Unbekannter KI-Status
//! (People sagt nicht "Mensch"): `AiStatusUnknown`, wie bei der Hüllenprüfung. `true` an einer sensiblen oder
//! geschützten Gruppe: `update` ergibt `Error::AiNotAllowed { group }`. Widerspruch IN DER EINGABE ist
//! `Error::Invalid` (`Field::AiAllowed`): `create` mit `sensitive` und `ai_allowed` = `true`, ebenso ein Patch mit
//! `sensitive = Some(true)` und `ai_allowed = Some(true)` zugleich (H19; nichts geschrieben, auch nicht der Rest
//! des Patches). Eine KI legt keine Gruppe an.
//!
//! WIRKUNG wirksam "nein" BEIM SCHREIBEN (`Error::AiNotAllowed { group }`, nichts geschrieben; die handelnde
//! Person ist ein Mensch): `grant` an eine KI-Gruppe oder ein KI-Konto; `assign_role` einer KI-Rolle mit Geltung
//! g; Teilnahme und Einladung einer KI in Räumen der Gruppe, wenn ein Mensch sie einlädt.
//! WIRKUNG wirksam "nein" OHNE SCHREIBEN (B4, die B3-Logik auf KI übertragen): ein Recht einer KI an der Gruppe
//! (Weg (ii)) ZÄHLT NIE, egal woher es kommt: Neumarkierung Mensch → KI, unbekannter Status → später KI
//! (`grant` an Unbekannte ist erlaubt), Altdaten und Migration, `restore` einer Rolle, Lücke im Widerruf. Der
//! Eintrag trägt `InForce::KindMismatch` (bleibt stehen, sichtbar in `grants` bzw. bei den Zuordnungen für
//! Menschen mit `Read`: das ist der Befund; Rechte werden nicht gespiegelt, also kein Spiegel-Befund); `may_do`
//! antwortet `allowed: false` mit `Reason::Agent`; `GroupMembers` und Räume rechnen ihn nicht.
//!
//! KI SELBST (H15): `AiNotAllowed` erreicht eine fragende KI NIE, auch nicht in Räumen. Will eine KI selbst
//! einem Raum einer Gruppe mit wirksamem "nein" beitreten oder eine KI dorthin einladen, antwortet das Programm
//! wie für einen Raum, den sie nicht kennt (`NotFound` bzw. die allgemeine Ablehnung des Programms, ohne Grund);
//! so verrät die Antwort das Feld nicht. Im Trait braucht jeder Weg zu `AiNotAllowed` `Manage` oder ein Recht,
//! das eine KI nicht hat; die Rangfolge prüft das vorher.
//!
//! KANTEN (H18): Mitglieds-Wege (`add_member`, `invite`, Annehmen, `admit_member`) und WIRKSAME Kanten
//! (`BottomUp`/`TopDown`) bleiben bei der strengeren KI-Regel: `AiNesting` gilt unabhängig vom Feld und geht vor.
//! Eine `Single`-Kante einer KI-Gruppe (Blatt) unter eine Gruppe mit "nein" ist ERLAUBT und kein Fehler: sie
//! überträgt weder Mitglieder noch Rechte (`nesting.rs`), fällt also weder unter (i) noch unter (ii); und weil
//! "nein" die Voreinstellung ist, könnte eine KI-Gruppe sonst fast nirgends im Baum stehen. Der Wechsel auf
//! "nein" löst sie nicht.
//!
//! WECHSEL wirksam "ja" → "nein" wirkt SOFORT, wie STOP KI, gleich aus welchem Grund (Patch, sensibel, geschützt
//! geworden, auch über Kanten): der Anbieter widerruft im selben Schritt jedes Recht einer KI an der Gruppe
//! (`HistoryKind::GrantRevoked` je `Grant`, `RoleUnassigned` je KI-Rolle mit Geltung g) und meldet die Änderung
//! (`ChangeKind::Group`); Programme mit Räumen beenden daraufhin die Teilnahmen der KI (Token und Sitzung
//! widerrufen). Es werden KEINE "KI-Mitglieder" entfernt: in Menschengruppen gibt es keine. "nein" → "ja" ändert
//! nichts Bestehendes.
//!
//! ERZWUNGEN "nein" (keine Ablehnung, sonst blockierte die KI-Einstellung die Admin-Vergabe): nach JEDER
//! Zustandsänderung, nach der eine Gruppe mit gespeichertem "ja" sensibel oder geschützt ist, speichert der
//! Anbieter für JEDE so betroffene Gruppe im selben Schritt "nein", mit der Wirkung oben und dem Eintrag
//! `HistoryKind::AiAllowedChanged { allowed: false, forced: true }` in ihrem Protokoll. Das gilt für alle Wege,
//! die `Protection::admin` ändern (Hülle eingeschlossen), nicht für eine Liste: u. a. `set_role`, `add_member`,
//! `invite`/Annehmen, `admit_member` mit Admin-Rolle, `assign_role`, `role_update`, `role_nest`,
//! `answer_role_request`, `change_role_nesting_mode`, `role_restore`, `request_nesting` (`Linked`),
//! `answer_nesting_request`, `change_nesting_mode`, `restore`, `update(sensitive = true)`; betroffen sind auch
//! Gruppen, die dadurch über die Hülle admin-wirksam werden (`BottomUp`-Kinder einer Admin-Gruppe). Fehlt der
//! gespeicherte Eintrag (Altdaten, Fehler), gilt trotzdem der abgeleitete wirksame Wert.
//!
//! RESTORE (H16): `restore` lehnt wegen des Feldes NIE ab. Rechte einer KI an einer Gruppe, die nach dem
//! Wiederherstellen wirksam "nein" ist, bleiben weg (nicht zurückgeholt) und stehen je Recht als
//! `GrantRevoked` bzw. `RoleUnassigned` im Protokoll (wie `NestingRemoved`); ist die Gruppe danach geschützt,
//! gilt zusätzlich ERZWUNGEN "nein".
//!
//! UNSICHTBAR FÜR KI (0054 Nachtrag 2, Präzisierung 1 und 2; H14): eine fragende KI liest `Group::ai_allowed` und
//! `GroupSummary::ai_allowed` als `None`; Einträge `AiAllowedChanged` (und Änderungen, die nur das Feld
//! betreffen, auch die dadurch ausgelösten `GrantRevoked`/`RoleUnassigned` an KI-Inhabern) fehlen für sie in
//! `history`, `changes` und im Export. Menschen lesen `Some(wirksamer Wert)`.
//!
//! RANGFOLGE (H17): `NotFound` (verborgen) vor `Forbidden`/`RoleForbidden` vor `AgentNotAdmin`/`AiStatusUnknown`
//! (wer handelt) vor `AppointerRequired`/`SecondAppointerRequired` vor `AiNesting`/`AiRoleForbidden` vor
//! `AgentNotAdmin` (ZIEL: `Manage` an eine KI, jedes Recht einer KI an einer sensiblen oder geschützten Gruppe)
//! vor `AiNotAllowed` vor den Prüfungen des Inhalts (`Invalid`, `AlreadyThere`). Ausnahme: der Widerspruch IN
//! DER EINGABE (H19, `ai` bzw. `sensitive` mit `ai_allowed = true`) ist `Invalid` statt `AiNotAllowed`.
//!
//! PROTOKOLL: jede Änderung des gespeicherten Werts (auch die erzwungene) als `HistoryKind::AiAllowedChanged`, nie
//! in `Updated { fields }`; Menschen mit `Read` sehen sie. SPIEGEL: KEIN `MirrorStep`; Kanidm kennt das Feld nicht
//! (KI-Konten stehen dort ohnehin nicht, ADR 0056 Punkt 8); Leitquelle bleibt fs-groups, fs-access fragt frisch.

/// Voreinstellung von "KI erlaubt" beim Anlegen (Nutzerentscheidung 39: Opt-in).
pub const AI_ALLOWED_DEFAULT: bool = false;
