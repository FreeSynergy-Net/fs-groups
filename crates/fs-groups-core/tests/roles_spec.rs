//! The protection rule as a pure specification: a table and an exhaustive small property.

use fs_groups_core::protect::check_change;
use fs_groups_core::{Error, Member, Membership, Role};
use fs_objects::ObjectId;

type Row<'a> = (&'a [(&'a str, &'a str, bool)], usize, &'a str);

fn ms(roles: &[(&str, &str, bool)]) -> Vec<Membership> {
    roles
        .iter()
        .map(|(p, role, in_force)| Membership {
            id: ObjectId::new(),
            group: ObjectId::new(),
            member: Member::Person((*p).into()),
            role: (*role).into(),
            in_force: *in_force,
        })
        .collect()
}

#[test]
fn the_specification_in_a_table() {
    let g = ObjectId::new();
    // (before, drop the entry at this index, expected)
    let table: [Row; 6] = [
        (&[("a", "owner", true)], 0, "owner"),
        (&[("a", "owner", true), ("b", "owner", true)], 0, "ok"),
        (&[("a", "owner", true), ("b", "manager", true)], 1, "ok"),
        (&[("a", "manager", true)], 0, "manager"),
        (&[("a", "member", true)], 0, "ok"),
        // an owner who is not in force is no holder, so nothing is protected
        (&[("a", "owner", false)], 0, "ok"),
    ];
    for (before, i, want) in table {
        let b = ms(before);
        let mut a = b.clone();
        a.remove(i);
        let got = match check_change(g, &b, &a) {
            Ok(()) => "ok",
            Err(Error::LastOwner { .. }) => "owner",
            Err(Error::LastManager { .. }) => "manager",
            Err(e) => panic!("{e}"),
        };
        assert_eq!(got, want, "{before:?}");
    }
}

#[test]
fn property_no_single_removal_empties_the_owners() {
    // all groups of up to 4 members over the roles: a removal that leaves holders-before > 0 and
    // holders-after == 0 is always refused, everything else is accepted
    let roles = ["", "manager", "owner", "boss"];
    let g = ObjectId::new();
    for code in 0..4usize.pow(4) {
        let list: Vec<_> = (0..4)
            .map(|i| (format!("p{i}"), roles[(code >> (2 * i)) & 3]))
            .collect();
        let b = ms(&list
            .iter()
            .map(|(p, r)| (p.as_str(), *r, true))
            .collect::<Vec<_>>());
        for i in 0..4 {
            let mut a = b.clone();
            a.remove(i);
            let count = |l: &[Membership], f: fn(&Role) -> bool| {
                l.iter().filter(|m| f(&m.typed_role())).count()
            };
            let owner_lost =
                count(&b, |r| *r == Role::Owner) > 0 && count(&a, |r| *r == Role::Owner) == 0;
            let admin_lost = count(&b, Role::is_admin) > 0 && count(&a, Role::is_admin) == 0;
            assert_eq!(
                check_change(g, &b, &a).is_err(),
                owner_lost || admin_lost,
                "{list:?} drop {i}"
            );
        }
    }
}

#[test]
fn a_role_and_its_iri_map_both_ways() {
    for r in [
        Role::Member,
        Role::Manager,
        Role::Owner,
        Role::Legacy("boss".into()),
        Role::Legacy("a b/ü%".into()),
        Role::Legacy("https://example.org/roles/chair".into()),
        Role::Legacy(String::new()),
    ] {
        assert_eq!(Role::from_iri(&r.iri()), r, "{r:?}");
    }
    // every IRI comes back unchanged through a role
    for iri in [
        "https://example.org/roles/x",
        "urn:x:y",
        "https://freesynergy.net/ns#role/owner",
        "https://freesynergy.net/ns#role/legacy/a%20b",
        "https://freesynergy.net/ns#role/legacy/%zz",
        "https://freesynergy.net/ns#role/nothing",
    ] {
        assert_eq!(Role::from_iri(iri).iri(), iri, "{iri}");
    }
}
