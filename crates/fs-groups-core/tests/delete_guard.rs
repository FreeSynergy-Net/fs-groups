//! `delete` has no unchecked form: the repository's own actor must be allowed, and the reason is said.

mod common;

use common::{ME, person, repo};
use fs_groups_core::{Error, Member, Role};

#[tokio::test]
async fn delete_asks_the_actor_and_says_why_it_refuses() {
    let r = repo().await;
    // no members: the creator (the actor) may delete
    let g = r.create("plain", "").await.unwrap();
    r.delete(g.id).await.unwrap();
    // an owner exists and the actor is not it: refused with the reason in the text
    let g = r.create("owned", "").await.unwrap();
    r.add_role(g.id, Member::Person(person("o")), &Role::Owner)
        .await
        .unwrap();
    let e = r.delete(g.id).await.unwrap_err();
    assert!(
        matches!(e, Error::Forbidden(ref t) if t.contains("only an owner")),
        "{e}"
    );
    assert!(r.get(g.id).await.unwrap().is_some(), "nothing deleted");
    // the same actor as the owner may
    let r2 = fs_groups_core::GroupRepo::clone(&r);
    r2.delete_by(g.id, &person("o")).await.unwrap();
}

#[tokio::test]
async fn the_actor_who_is_the_owner_deletes_through_plain_delete() {
    let r = repo().await;
    let g = r.create("mine", "").await.unwrap();
    r.add_role(g.id, Member::Person(ME.into()), &Role::Owner)
        .await
        .unwrap();
    r.delete(g.id).await.unwrap();
    assert!(r.get(g.id).await.unwrap().is_none());
}

#[tokio::test]
async fn a_missing_group_is_not_found() {
    let r = repo().await;
    let e = r.delete(fs_objects::ObjectId::new()).await.unwrap_err();
    assert!(matches!(e, Error::NotFound { .. }), "{e}");
}
