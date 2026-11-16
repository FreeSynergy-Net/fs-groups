//! Gemeinsame Basistypen: Wiederverwendung von `fs-types` (>= 0.0.22), kein Stand-in mehr. Weiterblätter-Stelle
//! ist `PageCursor` unter seinem eigenen Namen (`fs_types::Place` wird ab v0.0.24 die Ortshülle).
//!
//! Pattern: Value Object (Basistypen aus fs-types; Eintritt nur über `parse`/Konstruktoren).

pub use fs_types::{
    BasicValue, Date, ObjectId, Page, PageCursor, PersonArt, PersonRef, Reference, Revision,
    TypeProblem, Utc,
};
