//! Entwurfsort für Trait-Deltas, die noch nicht gesperrt sind (Workflow trait-delta). Heute: `groups_v2`, das
//! Delta zu `groups_v1` (ADR 0095, Eigentümer und drei Verwaltungsstufen), mit seiner Probe.
//!
//! Pattern: Repository + Port (der Trait liegt in `groups_v2/`; die Konformitätsfälle K1–K36 in `conformance_v2`).
//!
//! Fester Ort nach dem Fixpunkt: das Träger-Crate von fs-groups (Regel 8); `groups_v1/` bleibt unberührt.

// Wie `groups_v1` (Quelle: das 2021er Probe-Crate): nicht umformatiert, damit die fünf byte-gleichen Dateien
// byte-gleich bleiben (`conformance_v2::pure` prüft es), und mit denselben Ausnahmen.
#[rustfmt::skip]
#[allow(missing_docs, clippy::doc_markdown, clippy::must_use_candidate, clippy::large_enum_variant, clippy::wildcard_imports)]
pub mod groups_v2;

#[cfg(test)]
mod conformance_v2;
