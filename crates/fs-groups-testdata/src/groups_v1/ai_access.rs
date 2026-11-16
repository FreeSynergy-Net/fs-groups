//! Gruppenfeld "KI erlaubt" (`Group::ai_allowed`; ADR 0054 Nachtrag 2, Nutzerentscheidung 39, 2026-11-16; Opus
//! `review-adr-0054-nachtrag2-opus.md`; Tabelle `groups_group`, Spalte `ai_allowed INTEGER NOT NULL DEFAULT 0`).
//!
//! Pattern: Specification (Bedeutung, Wirkung und Rangfolge stehen hier, jeder Anbieter rechnet sie gleich;
//! Konformitätstest) + Value Object (Voreinstellung).
//!
//! BEDEUTUNG: "KI erlaubt" ist NIE Mitgliedschaft. ADR 0082 (KI-Regel, `nesting.rs`) gilt weiter: ein KI-Konto
//! ist nie Mitglied einer Menschengruppe (`AiNesting`), auch nicht bei "ja". Das Feld umfasst genau
//! (i) KI-Teilnahme oder -Einladung in einer Unterhaltung, einem Raum oder einer Sitzung DER GRUPPE (0054
//! Nachtrag 1: Einladung = Teilnahme; die Programme mit Räumen lesen das Feld, Plan-Register, nicht hier) und
//! (ii) ein Recht einer KI-Gruppe oder eines KI-Kontos AN dieser Gruppe (`Groups::grant`, `rights.rs`).
//! "ja" ist nur eine Möglichkeit, keine Einladung und kein Recht: es gelten weiter Einladung, Zustimmung aller
//! Anwesenden, Vorwarnung Spätkommender und STOP KI (0054 Nachtrag 1). Der Programm-Schalter "KI-Zugang (MCP)"
//! geht vor (Programm AUS = nie KI); er liegt nicht in `groups_v1`.
//!
//! WERTE: Voreinstellung `false` (`AI_ALLOWED_DEFAULT`, Anlegen-Dialog "KI erlaubt: nein"). FEST "nein" an einer
//! sensiblen Gruppe (`sensitive`) und an einer Gruppe mit Admin-Recht (Begriff `nesting.rs`: in ihr ist über
//! `Membership.role` eine Admin-Rolle vergeben oder ihr ist eine admin-wirksame Rolle zugeordnet). An einer
//! KI-Gruppe (`ai`) ist das Feld sinnlos: `ai = true` mit `ai_allowed = true` ist `Error::Invalid`
//! (`Field::AiAllowed`), bei `create` wie bei `update`.
//!
//! SETZEN (`GroupDraft::ai_allowed`, `GroupPatch::ai_allowed`): nur ein MENSCH mit `Manage` an der Gruppe. Eine
//! KI hat nie `Manage` und bekommt dieselbe Antwort wie für jedes andere `Manage`-Feld (`Forbidden`, an einer
//! für sie verborgenen Gruppe `NotFound`); hielte sie es aus Altdaten: `AgentNotAdmin`. Unbekannter KI-Status
//! (People sagt nicht "Mensch"): `AiStatusUnknown`, wie bei der Hüllenprüfung. `true` an einer sensiblen oder
//! admin-wirksamen Gruppe: `update` ergibt `Error::AiNotAllowed { group }`, `create` (noch ohne Id) mit
//! `sensitive = true` ergibt `Error::Invalid` (`Field::AiAllowed`). Eine KI legt keine Gruppe an.
//!
//! WIRKUNG "nein" (`Error::AiNotAllowed { group }`, nichts geschrieben): `grant` an eine KI-Gruppe oder ein
//! KI-Konto; `restore` mit solchen Rechten (nichts zurückgeholt); Teilnahme und Einladung in Räumen der Gruppe
//! (Programme mit Räumen, gleicher Fehler). Mitglieds-Wege (`add_member`, `invite`, Annehmen, `admit_member`,
//! Kanten) bleiben bei der strengeren KI-Regel: `AiNesting` gilt unabhängig vom Feld und geht vor.
//!
//! WECHSEL "ja" → "nein" wirkt SOFORT, wie STOP KI: der Anbieter widerruft im selben Schritt jedes Recht einer
//! KI-Gruppe oder eines KI-Kontos an der Gruppe (`HistoryKind::GrantRevoked` je Recht) und meldet die Änderung
//! (`ChangeKind::Group`); Programme mit Räumen beenden daraufhin die Teilnahmen der KI (Token und Sitzung
//! widerrufen). Es werden KEINE "KI-Mitglieder" entfernt: in Menschengruppen gibt es keine. "nein" → "ja" ändert
//! nichts Bestehendes.
//!
//! ERZWUNGEN "nein" (keine Ablehnung, sonst blockierte die KI-Einstellung die Admin-Vergabe): wird eine Gruppe
//! mit "ja" zur Gruppe mit Admin-Recht (`set_role`/`add_member` mit Admin-Rolle, `assign_role` einer
//! admin-wirksamen Rolle, `role_update`/`role_nest`, die eine zugeordnete Rolle admin-wirksam machen, Zustimmen,
//! `restore`) oder sensibel (`update(sensitive = true)`), setzt der Anbieter IM SELBEN SCHRITT "nein", mit der
//! Wirkung oben und dem Eintrag `HistoryKind::AiAllowedChanged { allowed: false, forced: true }`.
//!
//! UNSICHTBAR FÜR KI (0054 Nachtrag 2, Präzisierung 1 und 2): eine fragende KI liest `Group::ai_allowed` immer als
//! `false` und `GroupSummary::ai_allowed` als `None`; Einträge `AiAllowedChanged` (und Änderungen, die nur das
//! Feld betreffen) fehlen für sie in `history`, `changes` und im Export. `AiNotAllowed` erreicht eine KI nie:
//! jeder Weg dahin braucht `Manage` oder ein Recht, das sie nicht hat, und die Rangfolge prüft das vorher.
//!
//! RANGFOLGE: `NotFound` (verborgen) vor `Forbidden`/`RoleForbidden` vor `AgentNotAdmin`/`AiStatusUnknown` (wer
//! setzt) vor `AppointerRequired`/`SecondAppointerRequired` vor `AiNesting`/`AiRoleForbidden` vor
//! `AiNotAllowed` vor den Prüfungen des Inhalts (`Invalid`, `AlreadyThere`).
//!
//! PROTOKOLL: jede Änderung (auch die erzwungene) als `HistoryKind::AiAllowedChanged`, nie in
//! `Updated { fields }`; Menschen mit `Read` sehen sie. SPIEGEL: KEIN `MirrorStep`; Kanidm kennt das Feld nicht
//! (KI-Konten stehen dort ohnehin nicht, ADR 0056 Punkt 8); Leitquelle bleibt fs-groups, fs-access fragt frisch.

/// Voreinstellung von "KI erlaubt" beim Anlegen (Nutzerentscheidung 39: Opt-in).
pub const AI_ALLOWED_DEFAULT: bool = false;
