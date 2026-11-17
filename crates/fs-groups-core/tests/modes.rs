//! Modes per edge (`BottomUp` / `TopDown` / `Single`), and effective members with origin.

#![allow(clippy::many_single_char_names)] // short names of people and groups keep the cases readable

mod common;

use common::{person, repo, setup};
use fs_groups_core::{Group, GroupRepo, Member, NestingMode, Origin, RequestKind, Role};

async fn mk(r: &GroupRepo, n: &str) -> Group {
    r.create(n, "").await.unwrap()
}

async fn edge(r: &GroupRepo, child: &Group, parent: &Group, mode: NestingMode) {
    let q = r
        .request_parent_in(child.id, parent.id, mode)
        .await
        .unwrap();
    assert_eq!((q.kind, q.mode), (RequestKind::Link, mode));
    r.approve_parent(q.id).await.unwrap();
}

async fn join(r: &GroupRepo, g: &Group, n: &str) {
    r.add(g.id, Member::Person(person(n)), "").await.unwrap();
}

async fn who(r: &GroupRepo, g: &Group) -> Vec<String> {
    let mut v: Vec<_> = r
        .effective(g.id)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.person)
        .collect();
    v.sort();
    v
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|n| person(n)).collect()
}

#[tokio::test]
async fn bottom_up_counts_members_upward_only() {
    let r = repo().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    edge(&r, &kid, &top, NestingMode::BottomUp).await;
    join(&r, &top, "t").await;
    join(&r, &kid, "k").await;
    assert_eq!(who(&r, &top).await, names(&["k", "t"]));
    assert_eq!(who(&r, &kid).await, names(&["k"]));
}

#[tokio::test]
async fn top_down_counts_members_downward_only() {
    let r = repo().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    edge(&r, &kid, &top, NestingMode::TopDown).await;
    join(&r, &top, "t").await;
    join(&r, &kid, "k").await;
    assert_eq!(who(&r, &top).await, names(&["t"]));
    assert_eq!(who(&r, &kid).await, names(&["k", "t"]));
    assert_eq!(r.groups_of(&person("t")).await.unwrap().len(), 2);
    assert_eq!(r.groups_of(&person("k")).await.unwrap(), vec![kid.id]);
    assert!(r.is_member(kid.id, &person("t")).await.unwrap());
    assert!(!r.is_member(top.id, &person("k")).await.unwrap());
}

#[tokio::test]
async fn single_passes_nothing_either_way() {
    let r = repo().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    edge(&r, &kid, &top, NestingMode::Single).await;
    join(&r, &top, "t").await;
    join(&r, &kid, "k").await;
    assert_eq!(who(&r, &top).await, names(&["t"]));
    assert_eq!(who(&r, &kid).await, names(&["k"]));
    assert_eq!(r.groups_of(&person("k")).await.unwrap(), vec![kid.id]);
    assert_eq!(
        r.parents(kid.id).await.unwrap(),
        vec![top.id],
        "still in the tree"
    );
}

#[tokio::test]
async fn modes_mix_on_one_path_and_the_origin_names_the_source() {
    // low (BottomUp) in mid, high (TopDown) under mid: low -> mid -> high
    let r = repo().await;
    let (a, b, c) = (
        mk(&r, "low").await,
        mk(&r, "mid").await,
        mk(&r, "high").await,
    );
    edge(&r, &a, &b, NestingMode::BottomUp).await;
    edge(&r, &c, &b, NestingMode::TopDown).await;
    join(&r, &a, "x").await;
    join(&r, &b, "y").await;
    let eff = r.effective_origins(c.id).await.unwrap();
    let x = eff.iter().find(|e| e.person == person("x")).unwrap();
    assert_eq!(x.origin, Origin::Inherited(a.id));
    assert_eq!(x.via, vec![c.id, b.id, a.id]);
    let y = eff.iter().find(|e| e.person == person("y")).unwrap();
    assert_eq!(
        (y.origin, y.via.clone()),
        (Origin::Inherited(b.id), vec![c.id, b.id])
    );
    // a direct member says so
    join(&r, &c, "z").await;
    let z = r
        .effective_origins(c.id)
        .await
        .unwrap()
        .into_iter()
        .find(|e| e.person == person("z"))
        .unwrap();
    assert_eq!((z.origin, z.via), (Origin::Direct, vec![c.id]));
}

#[tokio::test]
async fn several_parents_and_paths_one_person_once_with_alternatives() {
    // diamond: bottom <- left, right <- top; x is in left and right
    let r = repo().await;
    let (top, left, right) = (
        mk(&r, "top").await,
        mk(&r, "left").await,
        mk(&r, "right").await,
    );
    edge(&r, &left, &top, NestingMode::BottomUp).await;
    edge(&r, &right, &top, NestingMode::BottomUp).await;
    join(&r, &left, "x").await;
    join(&r, &right, "x").await;
    join(&r, &top, "x").await;
    let eff = r.effective_origins(top.id).await.unwrap();
    assert_eq!(eff.len(), 1);
    assert_eq!(eff[0].origin, Origin::Direct, "direct wins over inherited");
    assert_eq!(eff[0].also.len(), 2);
}

#[tokio::test]
async fn old_edges_without_a_mode_are_bottom_up() {
    let (r, store) = setup().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    common::force_parent(&store, kid.id, top.id).await;
    join(&r, &kid, "k").await;
    assert_eq!(
        r.parent_mode(kid.id, top.id).await.unwrap(),
        NestingMode::BottomUp
    );
    assert_eq!(who(&r, &top).await, names(&["k"]));
}

#[tokio::test]
async fn a_new_child_gets_the_default_mode_of_the_parent() {
    let r = repo().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    r.set_default_mode(top.id, NestingMode::TopDown)
        .await
        .unwrap();
    let q = r.request_parent(kid.id, top.id).await.unwrap();
    assert_eq!(q.mode, NestingMode::TopDown);
    r.approve_parent(q.id).await.unwrap();
    assert_eq!(
        r.parent_mode(kid.id, top.id).await.unwrap(),
        NestingMode::TopDown
    );
    // the default only preselects: it does not touch an existing edge
    r.set_default_mode(top.id, NestingMode::Single)
        .await
        .unwrap();
    assert_eq!(
        r.parent_mode(kid.id, top.id).await.unwrap(),
        NestingMode::TopDown
    );
}

#[tokio::test]
async fn owners_inherit_as_members_not_as_owners() {
    let r = repo().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    edge(&r, &kid, &top, NestingMode::BottomUp).await;
    r.add_role(kid.id, Member::Person(person("o")), &Role::Owner)
        .await
        .unwrap();
    // the owner of the child is an effective member of the parent, but holds no role there
    assert_eq!(who(&r, &top).await, names(&["o"]));
    assert_eq!(r.direct(top.id).await.unwrap(), vec![]);
}

#[test]
fn mode_text_round_trips_and_arcs_are_directed() {
    for m in [
        NestingMode::BottomUp,
        NestingMode::TopDown,
        NestingMode::Single,
    ] {
        assert_eq!(NestingMode::parse(m.as_str()), Some(m));
    }
    assert_eq!(NestingMode::parse("up"), None);
    assert_eq!(NestingMode::BottomUp.arc(1, 2), Some((1, 2)));
    assert_eq!(NestingMode::TopDown.arc(1, 2), Some((2, 1)));
    assert_eq!(NestingMode::Single.arc(1, 2), None);
}
