//! Role references and modes map both ways without loss.

use fs_groups_core::Role;
use fs_groups_testdata::groups_v1::{NestingMode, RoleRef};
use fs_groups_testdata::{mode_from_contract, mode_to_contract, ref_to_role, role_to_ref};
use fs_types::{BasicValue, Reference};

#[test]
fn every_core_role_survives_the_contract_and_back() {
    for role in [
        Role::Member,
        Role::Manager,
        Role::Owner,
        Role::Legacy("boss".into()),
        Role::Legacy("a b/ü%".into()),
        Role::Legacy("https://example.org/roles/chair".into()),
        Role::Legacy(String::new()),
    ] {
        let reference = role_to_ref(&role).unwrap();
        assert_eq!(ref_to_role(&reference), role, "{role:?}");
    }
}

#[test]
fn every_contract_reference_survives_the_core_and_back() {
    for iri in [
        "https://example.org/roles/x",
        "urn:x:y",
        "https://freesynergy.net/ns#role/owner",
        "https://freesynergy.net/ns#role/manager",
        "https://freesynergy.net/ns#role/member",
        "https://freesynergy.net/ns#role/legacy/a%20b",
        "https://freesynergy.net/ns#role/other",
    ] {
        let reference = RoleRef(Reference::parse(iri).unwrap());
        let back = role_to_ref(&ref_to_role(&reference)).unwrap();
        assert_eq!(back, reference, "{iri}");
    }
}

#[test]
fn known_roles_have_fixed_references() {
    let owner = RoleRef(Reference::parse("https://freesynergy.net/ns#role/owner").unwrap());
    assert_eq!(ref_to_role(&owner), Role::Owner);
    assert_eq!(role_to_ref(&Role::Owner).unwrap(), owner);
}

#[test]
fn modes_map_variant_by_variant_both_ways() {
    for m in [
        NestingMode::BottomUp,
        NestingMode::TopDown,
        NestingMode::Single,
    ] {
        assert_eq!(mode_to_contract(mode_from_contract(m)), m);
    }
    for m in [
        fs_groups_core::NestingMode::BottomUp,
        fs_groups_core::NestingMode::TopDown,
        fs_groups_core::NestingMode::Single,
    ] {
        assert_eq!(mode_from_contract(mode_to_contract(m)), m);
    }
}
