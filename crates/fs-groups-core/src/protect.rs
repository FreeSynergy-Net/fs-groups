//! The protection of the last owner and the last manager, in one place.
//!
//! Pattern: Specification — a pure check of a membership change: it compares the group's holders
//! before and after. Every operation that could take an owner or manager away (remove, demote,
//! leave) builds the "after" list and asks here; nothing else decides it. Entries that are not in
//! force (an agent with an admin role) are no holders.

use fs_objects::ObjectId;

use crate::error::{Error, Result};
use crate::model::{Membership, Role};

/// The people who hold a role in force, each once.
fn holders(list: &[Membership], wanted: impl Fn(&Role) -> bool) -> Vec<&str> {
    let mut out: Vec<&str> = vec![];
    for m in list.iter().filter(|m| m.in_force) {
        if let (true, crate::model::Member::Person(p)) = (wanted(&m.typed_role()), &m.member) {
            if !out.contains(&p.as_str()) {
                out.push(p);
            }
        }
    }
    out
}

/// Refuse a change that leaves a group without an owner it had, or without a manager it had.
///
/// An owner counts as manager too, so a group with an owner is only ever refused for its last
/// owner; a group with managers but no owner (older data) is refused for its last manager.
/// `gone` names the person who would have been lost, for the message.
pub fn check_change(group: ObjectId, before: &[Membership], after: &[Membership]) -> Result<()> {
    let owners = |l| holders(l, |r| *r == Role::Owner);
    let admins = |l| holders(l, Role::is_admin);
    let lost =
        |b: Vec<&str>, a: &[&str]| b.into_iter().find(|p| !a.contains(p)).map(str::to_string);
    let (ob, oa) = (owners(before), owners(after));
    if !ob.is_empty() && oa.is_empty() {
        let person = lost(ob, &oa).unwrap_or_default();
        return Err(Error::LastOwner { group, person });
    }
    let (ab, aa) = (admins(before), admins(after));
    if !ab.is_empty() && aa.is_empty() {
        let person = lost(ab, &aa).unwrap_or_default();
        return Err(Error::LastManager { group, person });
    }
    Ok(())
}
