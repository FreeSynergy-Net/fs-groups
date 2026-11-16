//! Typisierte Fehler des Ordners `groups_v1/`. Jede Ursache ist eine Variante mit typisierten Zusatzfeldern, nie
//! ein Text. `Display` dient nur Entwicklern (englisch); die Person sieht Übersetzungen über die Kennungen
//! (ADR 0057).
//!
//! Pattern: Error Enum (typisierte Ursache + Zusatzfelder), `Display` nur zur Anzeige.

use super::shared::{ObjectId, PersonArt, Reference, Revision, TypeProblem, Utc};
use super::{InvitationState, RequestState, Right};
use std::fmt;

/// Wovon etwas nicht gefunden wurde. Kanten und Anfragen des Rollenbaums: `Nesting`, `NestingRequest`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Missing {
    Group(ObjectId),
    Membership(ObjectId),
    Invitation(ObjectId),
    Nesting(ObjectId),
    NestingRequest(ObjectId),
    Grant(ObjectId),
    Role(ObjectId),
    RoleAssignment(ObjectId),
    Block(ObjectId),
    /// Ein `RoleRef` (z. B. `Membership.role`) zeigt auf keine Rolle dieses Anbieters.
    RoleReference(Reference),
    /// Der Verweis zeigt auf nichts, das People kennt (oder auf eine gelöschte Person, AS2 `Tombstone`).
    Person(Reference),
}

/// Feld einer Eingabe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Field {
    Name,
    Kind,
    Description,
    Person,
    Role,
    Holder,
    Expires,
    Mirror,
    Abzug,
    /// Schlüssel einer Rolle (`Role::key`).
    Key,
    /// Geltung einer Zuordnung (`within`).
    Within,
    Managers,
    Concept,
    /// Letzter Tag einer Befristung oder Sperre (`until`).
    Until,
}

/// Was an einem Feld falsch ist.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProblemKind {
    Empty,
    TooLong {
        max: u32,
    },
    /// Steuer- oder unsichtbare Zeichen (Schreibweise wie heute `text.rs`).
    BadCharacters,
    BadReference(TypeProblem),
    /// Zeitpunkt oder Datum (`until`: vor heute in der Zone des Anbieters) liegt in der Vergangenheit.
    InPast,
    /// `mirror` bei `sensitive`.
    SensitiveNotMirrored,
    /// `mirror` bei `ai`: KI-Gruppen werden nie nach Kanidm gespiegelt (ADR 0056 Punkt 8: in Kanidm nur Menschen).
    AiNotMirrored,
    /// `abzug` ist in diesem Thema nie erlaubt.
    NoAbzug,
    /// Rolle passt nicht zur Geltung (`RoleScope::Group(g)` außerhalb von g; `within` an einer Gruppe, die es
    /// nicht gibt).
    OutOfScope,
}

/// Ein Prüffehler an einem Feld.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProblem {
    pub field: Field,
    pub problem: ProblemKind,
}

/// Warum etwas gerade nicht erreichbar ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unreachable {
    Store,
    Server,
    People,
    /// fs-admission (`SelfManagement`).
    Admission,
}

/// Art der Speicherstörung (wie `std::io::ErrorKind`, ohne Text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageKind {
    NotFound,
    PermissionDenied,
    NoSpace,
    Corrupt,
    Locked,
    Encrypted,
    Other,
}

/// Warum ein Modus (oder ein Schutz-Kennzeichen) nicht passt.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConflictKind {
    /// Ein Bogen des Mitgliedsgraphen (`from` wäre Mitglied von `to`) verbände eine sensible mit einer nicht
    /// sensiblen Gruppe, in EINER der beiden Richtungen (DSGVO Art. 9): sensibel nach nicht sensibel machte
    /// Besonderes offen, nicht sensibel nach sensibel machte Menschen ohne eigenen Beitritt zu wirksamen
    /// Mitgliedern. Gilt für Kanten, Moduswechsel, `restore` und `update(sensitive)`; `Single` ist erlaubt.
    /// Ebenso im Rollenbaum und für Zuordnungen Gruppe → Rolle (`from` = Gruppe, `to` = Rolle; beide gleich
    /// `sensitive`, sonst dieser Fehler).
    SensitiveFlow { from: ObjectId, to: ObjectId },
    /// Rollen-Kante in eine Rolle mit `RoleScope::Group(g)` aus einer Rolle anderer Geltung.
    ScopeMismatch { from: ObjectId, to: ObjectId },
    /// Das Ziel (Gruppe oder Rolle) ist selbstverwaltet (fs-admission, `SelfManagement`) und würde KI, oder
    /// geschützt (admin-wirksam, sensibel), ohne dass die Aufnahme-Regel passt (`Confirm`/`Vote` durch Inhaber von
    /// `admin_appointer`, keine Gruppen-Links, mindestens ein Inhaber; sensibel: `Confirm`/`Vote` ohne Links;
    /// Entscheidung 31, `appoint.rs`). Ausgelöst von `update`, `role_update`, `assign_role`,
    /// `add_member`/`set_role` mit Admin-Rolle, Rollen-Kante, Gruppen-Kante und Moduswechsel (jede Gruppe, die
    /// dadurch geschützt würde), Zustimmen, `restore`/`role_restore`. Die Gegenrichtung prüft fs-admission über
    /// `Roles::protection`.
    SelfManaged { target: ObjectId },
    /// Die Systemrolle `admin_appointer` lässt sich nicht löschen und ihre Art nicht ändern (`appoint.rs`).
    SystemRole { role: ObjectId },
}

/// Fehler des Themas.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    NotFound(Missing),
    /// Vergleich-und-Setzen gescheitert: jemand hat vorher geschrieben.
    Changed {
        current: Revision,
    },
    /// Die fragende Person hat dieses Recht an der Gruppe nicht (`Read` auch für `Visibility`).
    Forbidden {
        group: ObjectId,
        right: Right,
    },
    /// Die Kante schlösse einen Kreis im Mitgliedsgraphen (nach Umkehrung der `TopDown`-Kanten) oder im Baum
    /// (alle Modi, auch `Single`); `path` nennt ihn, erste Gruppe = letzte Gruppe.
    Cycle {
        path: Vec<ObjectId>,
    },
    /// Eine Kette (Baum oder Mitgliedsgraph) würde länger als `MAX_NESTING_DEPTH`.
    TooDeep {
        max: u32,
    },
    /// Modus oder Schutz passen nicht zusammen; nichts geschrieben. Nie `Forbidden`: das ist ein Rechtefehler.
    ModeConflict(ConflictKind),
    /// KI-Regel beim SCHREIBEN verletzt (Nutzerentscheidung 2026-11-16, ADR 0054, 0056): ein KI-Konto in einer
    /// Menschengruppe, ein Mensch in einer KI-Gruppe, `ai: false` an einer KI-Gruppe mit KI-Mitgliedern (je `path`
    /// = nur die Gruppe), eine KI-Gruppe mit wirksamer Kante (BottomUp/TopDown) oder mit Kindern (`path` = Kind,
    /// Eltern). `group` = die Gruppe, an der es scheitert. Nichts geschrieben, nie still. Altdaten und
    /// Neumarkierung sind KEIN Fehler: die Mitgliedschaft zählt nur nicht (`InForce::KindMismatch`).
    AiNesting {
        group: ObjectId,
        path: Vec<ObjectId>,
    },
    /// Falsche Art im Verweis: eine Gruppe als Mitglied (dafür `request_nesting`), eine Organisation als
    /// Eingeladene usw.
    WrongArt {
        field: Field,
        got: PersonArt,
    },
    /// Die fragende Person ist nicht die eingeladene.
    NotInvitee,
    /// Die Einladung ist abgelaufen (beim Antworten als `Expired` gespeichert).
    Expired {
        at: Utc,
    },
    /// Einladung nicht mehr offen (angenommen, abgelehnt, widerrufen, abgelaufen).
    InvitationClosed(InvitationState),
    /// Verschachtelungs-Anfrage nicht mehr offen.
    RequestClosed(RequestState),
    /// Kante, offene Anfrage, Modus, Mitgliedschaft, Zuordnung, Sperre oder Rollen-Schlüssel (über Instanz und
    /// Gruppen, `Role::key`) gibt es schon in dieser Form (trägt die bestehende Id).
    AlreadyThere(ObjectId),
    /// Die fragende Person hat dieses Recht an der Rolle nicht (`Read` oder `Manage`, `role_api.rs`).
    RoleForbidden {
        role: ObjectId,
        right: Right,
    },
    /// KI-Kette der Rollen beim SCHREIBEN verletzt (`role_holders.rs`): KI-Konto oder KI-Gruppe mit einer
    /// Nicht-KI-Rolle, Mensch mit einer KI-Rolle, KI-Rolle mit wirksamer Kante oder als Eltern, oder eine Kette
    /// von einer KI zu einer Rolle mit `admin`/`sensitive`. `role` = Rolle, an der es scheitert; `path` = Inhaber-
    /// Gruppe (falls vorhanden), dann Rollen-Ids bis zur verbotenen (verborgene fehlen). Nichts geschrieben.
    AiRoleForbidden {
        role: ObjectId,
        path: Vec<ObjectId>,
    },
    /// Eine Person bekäme eine admin-wirksame Rolle neu oder länger (Aufnehmen in eine geschützte Gruppe, Rolle
    /// vergeben, Kante, Verlängern, Wiederherstellen, Annehmen gegen das Recht der einladenden Person), ohne dass
    /// die handelnde Person "Admins benennen" (`admin_appointer`) wirksam und mit passender GELTUNG hält
    /// (Entscheidung 31, alle Wege in `appoint.rs`); `target` = Gruppe bzw. Rolle.
    AppointerRequired {
        target: ObjectId,
    },
    /// Selbstvergabe (`appoint.rs`): die handelnde Person bekäme selbst eine admin-wirksame Rolle neu oder länger
    /// (auch `admin_appointer`, auch Selbstaufnahme in eine geschützte Gruppe); ein ZWEITER Inhaber muss handeln
    /// oder zustimmen. Nicht für den Installierenden. Nichts geschrieben.
    SecondAppointerRequired {
        target: ObjectId,
    },
    /// Admin-Rolle oder `Manage` für eine als KI markierte Person (ADR 0056 Punkt 6); auch: eine KI legt eine
    /// Rolle an, oder `Role::managers` wäre eine KI-Gruppe.
    AgentNotAdmin,
    /// Admin-Rolle oder `Manage`, aber People sagt nicht, ob die Person ein Mensch ist (im Zweifel nein); auch:
    /// eine Person mit unbekanntem Status soll in eine KI-Gruppe (in Menschengruppen erlaubt) oder bekommt direkt
    /// eine admin-wirksame Rolle oder eine KI-Rolle.
    AiStatusUnknown,
    /// `restore` nach Ablauf der Frist oder für eine nicht gelöschte Gruppe.
    NotRestorable(ObjectId),
    /// Die letzte Person mit `Manage` kann sich nicht selbst entfernen und nicht gesperrt werden; die Gruppe bzw.
    /// Rolle bliebe ohne Verwaltung. Der Installierende ist nie sperrbar (`block` gegen ihn: dieser Fehler, B2).
    LastManager,
    /// Prüfung gescheitert; nichts geschrieben.
    Invalid(Vec<FieldProblem>),
    Unavailable(Unreachable),
    Storage(StorageKind),
}
pub type Result<T> = std::result::Result<T, Error>;

impl From<TypeProblem> for Error {
    fn from(p: TypeProblem) -> Self {
        Self::Invalid(vec![FieldProblem {
            field: Field::Person,
            problem: ProblemKind::BadReference(p),
        }])
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(m) => write!(f, "not found: {m:?}"),
            Self::Changed { current } => {
                write!(f, "changed meanwhile (now revision {})", current.0)
            }
            Self::Forbidden { right, .. } | Self::RoleForbidden { right, .. } => {
                write!(f, "not allowed: needs {right:?}")
            }
            Self::Cycle { path } => write!(f, "would close a cycle of {} groups", path.len()),
            Self::TooDeep { max } => write!(f, "nesting deeper than {max} levels"),
            Self::WrongArt { field, got } => write!(f, "{field:?} may not be a {got:?}"),
            Self::Expired { at } => write!(f, "invitation expired at {} ms", at.millis()),
            Self::Invalid(p) => write!(f, "invalid input: {p:?}"),
            other => write!(f, "{other:?}"),
        }
    }
}
impl std::error::Error for Error {}
