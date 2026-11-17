//! Typisierte Fehler des Ordners `groups_v2/`. Jede Ursache ist eine Variante mit typisierten Zusatzfeldern, nie
//! ein Text. `Display` dient nur Entwicklern (englisch); die Person sieht Übersetzungen über die Kennungen
//! (ADR 0057).
//!
//! Pattern: Error Enum (typisierte Ursache + Zusatzfelder), `Display` nur zur Anzeige.

use super::shared::{ObjectId, PersonArt, PersonRef, Reference, Revision, TypeProblem, Utc};
use super::{InvitationState, RequestState, Right, Scope};
use std::fmt;

/// Wovon etwas nicht gefunden wurde.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    Group(ObjectId),
    Membership(ObjectId),
    Invitation(ObjectId),
    Nesting(ObjectId),
    NestingRequest(ObjectId),
    Grant(ObjectId),
    /// Der Verweis zeigt auf nichts, das People kennt (oder auf eine gelöschte Person, AS2 `Tombstone`).
    Person(Reference),
}

/// Feld einer Eingabe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// Was an einem Feld falsch ist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProblemKind {
    Empty,
    TooLong {
        max: u32,
    },
    /// Steuer- oder unsichtbare Zeichen (Schreibweise wie heute `text.rs`).
    BadCharacters,
    BadReference(TypeProblem),
    /// Zeitpunkt liegt in der Vergangenheit.
    InPast,
    /// `mirror` bei `sensitive`.
    SensitiveNotMirrored,
    /// `mirror` bei `ai`: KI-Gruppen werden nie nach Kanidm gespiegelt (ADR 0056 Punkt 8: in Kanidm nur Menschen).
    AiNotMirrored,
    /// `abzug` ist in diesem Thema nie erlaubt.
    NoAbzug,
}

/// Ein Prüffehler an einem Feld.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProblem {
    pub field: Field,
    pub problem: ProblemKind,
}

/// Warum etwas gerade nicht erreichbar ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreachable {
    Store,
    Server,
    People,
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

/// Warum ein Modus (oder `sensitive`) nicht passt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictKind {
    /// Ein Bogen des Mitgliedsgraphen (`from` wäre Mitglied von `to`) verbände eine sensible mit einer nicht
    /// sensiblen Gruppe, in EINER der beiden Richtungen (DSGVO Art. 9): sensibel nach nicht sensibel machte
    /// Besonderes offen, nicht sensibel nach sensibel machte Menschen ohne eigenen Beitritt zu wirksamen
    /// Mitgliedern. Gilt für Kanten, Moduswechsel, `restore` und `update(sensitive)`; `Single` ist erlaubt.
    SensitiveFlow { from: ObjectId, to: ObjectId },
}

/// Welche Handlung den Schutz der letzten auslöste (das Begründungsfeld von `LastOwner`/`LastManager`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuardWhy {
    /// Entfernen durch eine andere Person (`remove_member`).
    Remove,
    /// Austritt (`remove_member` durch die Person selbst).
    Leave,
    /// Herabstufen per `set_role`.
    Demote,
    /// `revoke_grant`.
    RevokeGrant,
    /// Sperren (`block`).
    Block,
}

/// Fehler des Themas.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Kante, offene Anfrage, Modus oder Mitgliedschaft gibt es schon in dieser Form (trägt die bestehende Id).
    AlreadyThere(ObjectId),
    /// Admin-Rolle oder `Manage` für eine als KI markierte Person (ADR 0056 Punkt 6).
    AgentNotAdmin,
    /// Admin-Rolle oder `Manage`, aber People sagt nicht, ob die Person ein Mensch ist (im Zweifel nein); auch:
    /// eine Person mit unbekanntem Status soll in eine KI-Gruppe (in Menschengruppen erlaubt).
    AiStatusUnknown,
    /// `restore` nach Ablauf der Frist oder für eine nicht gelöschte Gruppe.
    NotRestorable(ObjectId),
    /// Schutz der letzten (Specification, jeder Anbieter rechnet gleich): hatte die Gruppe mindestens einen
    /// Eigentümer und hätte danach keinen, `LastOwner`; sonst, hatte sie Verwaltung und hätte danach keine,
    /// `LastManager`. Eigentümer = Personen mit wirksamer, direkter Rolle `ROLE_OWNER` (`InForce::Yes`);
    /// Verwaltung = Personen, für die `may_do(.., Manage)` ohne `Reason::Intervention` ja ergäbe. Eingriffe
    /// von (a)/(b) laufen durch DIESELBE Prüfung; jede Person zählt einmal. `person` = die Person, die wegfiele.
    /// FAIL-CLOSED: nicht lesbarer Vorher-Stand = Speicherfehler, nichts geschrieben; Personenprüfung nicht
    /// erreichbar = `Unavailable(People)`; im Vorher zählen auch `AiStatusUnknown`, im Nachher nur `Yes`.
    LastOwner {
        group: ObjectId,
        person: PersonRef,
        why: GuardWhy,
    },
    /// Wie `LastOwner`, für die Verwaltung (in v1 eine Einheit-Variante).
    LastManager {
        group: ObjectId,
        person: PersonRef,
        why: GuardWhy,
    },
    /// `grant(.., Own)`: `Own` gibt es nur über die Rolle `ROLE_OWNER`.
    OwnViaRoleOnly,
    /// Ein betreutes Konto soll Eigentümerin werden (Antwort 102): `create` (Entscheidung b7 Pkt 9: eine
    /// betreute Person legt keine Gruppe an, siehe `SupervisedCannotCreate`), `add_member`, `set_role`,
    /// `invite`, `transfer_ownership` oder Leerstelle. Nichts geschrieben.
    SupervisedCannotOwn {
        person: PersonRef,
    },
    /// `create` durch ein betreutes Konto (Entscheidung b7 Pkt 9): abgelehnt, nichts angelegt. Die Anzeige ist
    /// Sache des Programms (de/en, ADR 0057): Kennung `groups.supervised-cannot-create` mit Grund (das Konto
    /// wird betreut) und Hinweis (`groups.supervised-cannot-create.hint`: "die verantwortliche Person kann die
    /// Gruppe anlegen"); `person` ist der Parameter. Kein Text im Fehler.
    SupervisedCannotCreate {
        person: PersonRef,
    },
    /// `is_supervised` ergab `None` bei einer Eigentümer-Vergabe oder beim Anlegen: abgelehnt (fail-closed).
    SupervisedStatusUnknown {
        person: PersonRef,
    },
    /// Das Recht gibt es nur über einen Bereich (`ScopedGrant`), und es ist kein Eingriff offen (nie geöffnet,
    /// geschlossen oder abgelaufen). Nichts geschrieben.
    InterventionRequired {
        scope: Scope,
    },
    /// Eingriff ohne Grund (`reason` leer).
    ReasonMissing,
    /// Der letzte `ScopedGrant` für `Project` lässt sich nicht entziehen.
    LastScope {
        scope: Scope,
    },
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
            Self::Forbidden { right, .. } => write!(f, "not allowed: needs {right:?}"),
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
