//! Changing the mode of an edge: consent, defaults, refusals.

#![allow(clippy::many_single_char_names)] // short names of people and groups keep the cases readable

mod common;

use common::{person, repo};
use fs_groups_core::{Error, Group, GroupRepo, Member, NestingMode, RequestKind};

async fn mk(r: &GroupRepo, n: &str) -> Group {
    r.create(n, "").await.unwrap()
}

async fn edge(r: &GroupRepo, child: &Group, parent: &Group, mode: NestingMode) {
    let q = r
        .request_parent_in(child.id, parent.id, mode)
        .await
        .unwrap();
    r.approve_parent(q.id).await.unwrap();
}

async fn join(r: &GroupRepo, g: &Group, n: &str) {
    r.add(g.id, Member::Person(person(n)), "").await.unwrap();
}

async fn who(r: &GroupRepo, g: &Group) -> Vec<String> {
    r.effective(g.id)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.person)
        .collect()
}

#[tokio::test]
async fn a_mode_change_needs_consent_except_to_single() {
    let r = repo().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    edge(&r, &kid, &top, NestingMode::BottomUp).await;
    join(&r, &kid, "k").await;
    // to TopDown: a request, nothing changes until it is approved
    assert!(
        matches!(r.set_single(kid.id, top.id).await.map(|_| ()), Ok(())),
        "single is at once"
    );
    assert_eq!(
        who(&r, &top).await,
        Vec::<String>::new(),
        "Single: nothing flows now"
    );
    let q = r
        .request_mode_change(kid.id, top.id, NestingMode::TopDown)
        .await
        .unwrap();
    assert_eq!(q.kind, RequestKind::ChangeMode);
    assert_eq!(
        r.parent_mode(kid.id, top.id).await.unwrap(),
        NestingMode::Single
    );
    assert_eq!(
        r.request_mode_change(kid.id, top.id, NestingMode::TopDown)
            .await
            .unwrap()
            .id,
        q.id
    );
    r.approve_parent(q.id).await.unwrap();
    assert_eq!(
        r.parent_mode(kid.id, top.id).await.unwrap(),
        NestingMode::TopDown
    );
    // a declined request changes nothing
    let q = r
        .request_mode_change(kid.id, top.id, NestingMode::BottomUp)
        .await
        .unwrap();
    r.decline_parent(q.id).await.unwrap();
    assert_eq!(
        r.parent_mode(kid.id, top.id).await.unwrap(),
        NestingMode::TopDown
    );
}

#[tokio::test]
async fn refused_mode_changes() {
    let r = repo().await;
    let (top, kid, other) = (mk(&r, "top").await, mk(&r, "kid").await, mk(&r, "o").await);
    edge(&r, &kid, &top, NestingMode::BottomUp).await;
    let same = r
        .request_mode_change(kid.id, top.id, NestingMode::BottomUp)
        .await;
    assert!(matches!(same, Err(Error::Conflict(_))));
    let to_single = r
        .request_mode_change(kid.id, top.id, NestingMode::Single)
        .await;
    assert!(
        matches!(to_single, Err(Error::Conflict(_))),
        "single needs no request"
    );
    let none = r
        .request_mode_change(kid.id, other.id, NestingMode::TopDown)
        .await;
    assert!(matches!(none, Err(Error::NotFound { .. })));
    assert!(matches!(
        r.set_single(kid.id, other.id).await,
        Err(Error::NotFound { .. })
    ));
    // a second link in any mode is a conflict; a cycle stays a cycle in every mode
    let again = r
        .request_parent_in(kid.id, top.id, NestingMode::Single)
        .await;
    assert!(matches!(again, Err(Error::Conflict(_))));
    for m in [
        NestingMode::BottomUp,
        NestingMode::TopDown,
        NestingMode::Single,
    ] {
        let c = r.request_parent_in(top.id, kid.id, m).await;
        assert!(matches!(c, Err(Error::Cycle { .. })), "{m:?}");
    }
}

#[tokio::test]
async fn removing_an_edge_forgets_its_mode_and_the_flow() {
    let r = repo().await;
    let (top, kid) = (mk(&r, "top").await, mk(&r, "kid").await);
    edge(&r, &kid, &top, NestingMode::TopDown).await;
    r.remove_parent(kid.id, top.id).await.unwrap();
    assert!(
        r.get(kid.id)
            .await
            .unwrap()
            .unwrap()
            .parent_modes
            .is_empty()
    );
    join(&r, &top, "t").await;
    assert_eq!(who(&r, &kid).await, Vec::<String>::new());
    // the same pair again starts from a clean slate
    edge(&r, &kid, &top, NestingMode::BottomUp).await;
    assert_eq!(
        r.parent_mode(kid.id, top.id).await.unwrap(),
        NestingMode::BottomUp
    );
}
