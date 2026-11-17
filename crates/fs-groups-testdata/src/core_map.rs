//! The lossless mapping between the contract types of `groups_v1` and the types of the program core.
//!
//! Pattern: Adapter (pure functions, both directions) — there is one model, the contract; the core
//! types are a form of it. `RoleRef` (an IRI) maps to `Role` and back through `Role::iri` /
//! `Role::from_iri`; `NestingMode` maps variant by variant. What the contract lacks (the owner
//! role as a fixed value, `LastOwner`, `transfer_ownership`, `effective_origins`) is a `groups_v2`
//! trait delta, not a second type world.

use fs_groups_core::{NestingMode as CoreMode, Role};
use fs_types::{BasicValue, TypeProblem};

use crate::groups_v1::{NestingMode, RoleRef};

/// A core role as the contract's role reference.
///
/// # Errors
/// Only for a legacy text that looks like an IRI but is not an acceptable `Reference`.
pub fn role_to_ref(role: &Role) -> Result<RoleRef, TypeProblem> {
    fs_types::Reference::parse(&role.iri()).map(RoleRef)
}

/// A contract role reference as a core role (never fails: an unknown IRI is a legacy role).
#[must_use]
pub fn ref_to_role(role: &RoleRef) -> Role {
    Role::from_iri(&role.0.canonical())
}

/// The contract's mode as the core's.
#[must_use]
pub fn mode_from_contract(mode: NestingMode) -> CoreMode {
    match mode {
        NestingMode::BottomUp => CoreMode::BottomUp,
        NestingMode::TopDown => CoreMode::TopDown,
        NestingMode::Single => CoreMode::Single,
    }
}

/// The core's mode as the contract's.
#[must_use]
pub fn mode_to_contract(mode: CoreMode) -> NestingMode {
    match mode {
        CoreMode::BottomUp => NestingMode::BottomUp,
        CoreMode::TopDown => NestingMode::TopDown,
        CoreMode::Single => NestingMode::Single,
    }
}
