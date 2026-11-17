//! The fixed role list, the protection of the last owner / manager, ownership transfer, migration.

#![allow(clippy::many_single_char_names)] // short names of people and groups keep the cases readable

mod common;

use common::{ME, person, repo, setup};
use fs_groups_core::{Error, GroupRepo, Member, Membership, Role};
use fs_objects::ObjectId;

async fn with(r: &GroupRepo, roles: &[(&str, Role)]) -> (ObjectId, Vec<Membership>) {
    let g = r.create("team", "team").await.unwrap();
    let mut out = vec![];
    for (n, role) in roles {
        out.push(
            r.add_role(g.id, Member::Person(person(n)), role)
                .await
                .unwrap(),
        );
    }
    (g.id, out)
}

#[test]
fn parse_maps_known_texts_and_keeps_the_rest_as_legacy() {
    for (t, want) in [
        ("", Role::Member),
        ("member", Role::Member),
        (" Mitglied ", Role::Member),
        ("admin", Role::Manager),
        ("Verwalter", Role::Manager),
        ("OWNER", Role::Owner),
        ("eigentümer", Role::Owner),
        ("Eigentuemer", Role::Owner),
        ("boss", Role::Legacy("boss".into())),
    ] {
        assert_eq!(Role::parse(t), want, "{t:?}");
    }
    assert_eq!(Role::Member.as_str(), "");
    assert_eq!(Role::parse("boss").as_str(), "boss");
    assert!(!Role::Legacy("x".into()).is_admin());
}

#[tokio::test]
async fn old_stored_role_texts_read_as_the_typed_list() {
    let r = repo().await;
    let g = r.create("old", "").await.unwrap();
    for (n, t) in [("a", "owner"), ("b", "admin"), ("c", ""), ("d", "boss")] {
        r.add(g.id, Member::Person(person(n)), t).await.unwrap();
    }
    let roles: Vec<_> = r
        .direct(g.id)
        .await
        .unwrap()
        .iter()
        .map(Membership::typed_role)
        .collect();
    assert_eq!(
        roles,
        [
            Role::Owner,
            Role::Manager,
            Role::Member,
            Role::Legacy("boss".into())
        ]
    );
}

#[tokio::test]
async fn the_typed_api_refuses_a_legacy_role() {
    let r = repo().await;
    let (g, m) = with(&r, &[("a", Role::Owner), ("b", Role::Member)]).await;
    let err = r
        .add_role(g, Member::Person(person("c")), &Role::Legacy("boss".into()))
        .await;
    assert!(matches!(err, Err(Error::UnknownRole(_))));
    let err = r.set_role(m[1].id, &Role::Legacy("boss".into())).await;
    assert!(matches!(err, Err(Error::UnknownRole(_))));
}

#[tokio::test]
async fn the_last_owner_cannot_be_removed_demoted_or_leave() {
    let r = repo().await;
    let (g, m) = with(
        &r,
        &[
            ("a", Role::Owner),
            ("b", Role::Manager),
            ("c", Role::Member),
        ],
    )
    .await;
    assert!(matches!(
        r.remove(m[0].id).await,
        Err(Error::LastOwner { .. })
    ));
    assert!(matches!(
        r.set_role(m[0].id, &Role::Manager).await,
        Err(Error::LastOwner { .. })
    ));
    assert!(matches!(
        r.set_role(m[0].id, &Role::Member).await,
        Err(Error::LastOwner { .. })
    ));
    assert!(matches!(
        r.leave(g, &person("a")).await,
        Err(Error::LastOwner { .. })
    ));
    assert_eq!(r.direct(g).await.unwrap().len(), 3, "nothing changed");
    // the others can go: the owner remains the admin
    r.remove(m[1].id).await.unwrap();
    r.leave(g, &person("c")).await.unwrap();
}

#[tokio::test]
async fn with_a_second_owner_the_first_can_go() {
    let r = repo().await;
    let (g, m) = with(&r, &[("a", Role::Owner), ("b", Role::Owner)]).await;
    r.leave(g, &person("a")).await.unwrap();
    assert!(matches!(
        r.remove(m[1].id).await,
        Err(Error::LastOwner { .. })
    ));
}

#[tokio::test]
async fn without_an_owner_the_last_manager_is_protected() {
    let r = repo().await;
    let (g, m) = with(
        &r,
        &[
            ("a", Role::Manager),
            ("b", Role::Manager),
            ("c", Role::Member),
        ],
    )
    .await;
    r.remove(m[0].id).await.unwrap();
    assert!(matches!(
        r.remove(m[1].id).await,
        Err(Error::LastManager { .. })
    ));
    assert!(matches!(
        r.set_role(m[1].id, &Role::Member).await,
        Err(Error::LastManager { .. })
    ));
    assert!(matches!(
        r.leave(g, &person("b")).await,
        Err(Error::LastManager { .. })
    ));
}

#[tokio::test]
async fn a_group_with_only_plain_members_is_not_protected() {
    let r = repo().await;
    let (g, m) = with(&r, &[("a", Role::Member)]).await;
    r.remove(m[0].id).await.unwrap();
    assert!(matches!(
        r.leave(g, &person("a")).await,
        Err(Error::NotMember { .. })
    ));
}

#[tokio::test]
async fn set_role_changes_the_role_and_merges_a_twin() {
    let r = repo().await;
    let (g, m) = with(&r, &[("a", Role::Owner), ("b", Role::Member)]).await;
    let up = r.set_role(m[1].id, &Role::Manager).await.unwrap();
    assert_eq!(up.typed_role(), Role::Manager);
    assert_eq!(r.direct(g).await.unwrap()[1].typed_role(), Role::Manager);
    // back to member, then a second member membership is not made twice
    r.set_role(m[1].id, &Role::Member).await.unwrap();
    assert_eq!(r.direct(g).await.unwrap().len(), 2);
}

#[tokio::test]
async fn an_agent_cannot_be_made_manager_or_owner() {
    let (r, _s) = setup().await;
    let bot = person("bot");
    let r = r.with_ai_marking(common::Agents::with(&[bot.as_str()]));
    let (g, m) = with(&r, &[("a", Role::Owner), ("bot", Role::Member)]).await;
    assert!(matches!(
        r.set_role(m[1].id, &Role::Manager).await,
        Err(Error::AgentNotAdmin(_))
    ));
    assert!(matches!(
        r.transfer_ownership(g, &person("a"), &bot, &Role::Manager)
            .await,
        Err(Error::AgentNotAdmin(_))
    ));
}

#[tokio::test]
async fn ownership_moves_in_one_step() {
    let r = repo().await;
    let (g, _) = with(
        &r,
        &[("a", Role::Owner), ("b", Role::Member), ("c", Role::Member)],
    )
    .await;
    let new = r
        .transfer_ownership(g, &person("a"), &person("b"), &Role::Manager)
        .await
        .unwrap();
    assert_eq!(new.typed_role(), Role::Owner);
    let now: Vec<_> = r
        .direct(g)
        .await
        .unwrap()
        .iter()
        .map(Membership::typed_role)
        .collect();
    assert_eq!(now, [Role::Manager, Role::Owner, Role::Member]);
    // the former owner may now leave, the new one may not
    assert!(matches!(
        r.leave(g, &person("b")).await,
        Err(Error::LastOwner { .. })
    ));
    r.leave(g, &person("a")).await.unwrap();
}

#[tokio::test]
async fn a_transfer_needs_an_owner_a_member_and_a_sensible_role() {
    let r = repo().await;
    let (g, _) = with(&r, &[("a", Role::Owner), ("b", Role::Member)]).await;
    let (a, b, z) = (person("a"), person("b"), person("z"));
    let t = |f: &str, to: &str, k: Role| {
        let r = r.clone();
        let (f, to) = (f.to_string(), to.to_string());
        async move { r.transfer_ownership(g, &f, &to, &k).await }
    };
    assert!(matches!(
        t(&b, &a, Role::Member).await,
        Err(Error::Forbidden(_))
    ));
    assert!(matches!(
        t(&a, &z, Role::Member).await,
        Err(Error::NotMember { .. })
    ));
    assert!(matches!(
        t(&a, &a, Role::Member).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        t(&a, &b, Role::Owner).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        t(&a, &b, Role::Legacy("x".into())).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        r.direct(g).await.unwrap()[0].typed_role(),
        Role::Owner,
        "nothing changed"
    );
}

#[tokio::test]
async fn only_an_owner_deletes_a_group() {
    let r = repo().await;
    let (g, _) = with(
        &r,
        &[
            ("a", Role::Owner),
            ("b", Role::Manager),
            ("c", Role::Member),
        ],
    )
    .await;
    for who in ["b", "c", "z"] {
        let e = r.delete_by(g, &person(who)).await;
        assert!(matches!(e, Err(Error::Forbidden(_))), "{who}");
    }
    r.delete_by(g, &person("a")).await.unwrap();
    assert!(r.get(g).await.unwrap().is_none());
    // no owner at all (older data): a manager may; the installer always may
    let (g2, _) = with(&r, &[("b", Role::Manager)]).await;
    r.delete_by(g2, &person("b")).await.unwrap();
    let r2 = r.clone().with_installer(ME).unwrap();
    let (g3, _) = with(&r2, &[("a", Role::Owner)]).await;
    r2.delete_by(g3, ME).await.unwrap();
}
