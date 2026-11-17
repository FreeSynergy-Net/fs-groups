//! Konformitätsfälle K1–K36 von `groups_v2` (Entwurf Abschnitt 6, plus K36 = Entscheidung b7 Pkt 9): je Regel ein
//! Fall und der Mutant, der ihn rot macht. Skelett der Probe, kein Anbieter.
//!
//! Pattern: Template method (feste Fälle gegen einen `Groups`-Anbieter; `kern::Kern` sagt, wie man ihn in die
//! Zustände der Fälle bringt).
//!
//! Was LÄUFT: `pure` (K17 Rückweg der festen Rollen, Fähigkeitsliste, byte-gleiche v1-Dateien, ein
//! `MAX_NESTING_DEPTH`, Eingriffsfrist-Standard). Was `#[ignore = "kern: ..."]` trägt, braucht den v2-Anbieter
//! (`FakeGroups` v2 oder `LocalGroups`, Bahn Kern): der Anbieter fehlt, `Kern::start` ist `unimplemented!`, ein
//! ignorierter Fall wird dadurch nie still grün. Ist der Anbieter da, fällt das `ignore` weg und jeder Fall muss
//! rot werden, wenn man seinen Mutanten einbaut (Mutant-Lauf der Probe-Folge).

// The cases are flat lists of steps on a few groups and persons of ONE case (`p`, `q`, `m`); the short names are
// the persons, as in the v1 conformance rules.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

mod kern;
mod pure;
mod rules_guard;
mod rules_nesting;
mod rules_owner;
mod rules_scope;
mod rules_supervised;
