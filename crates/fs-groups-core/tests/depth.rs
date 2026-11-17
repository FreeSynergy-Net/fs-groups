//! The longest chain of nesting edges is 16, as in the contract `groups_v1`.

#![allow(clippy::many_single_char_names)] // short names of groups keep the cases readable

mod common;

use common::repo;
use fs_groups_core::{Error, Group, GroupRepo, MAX_NESTING_DEPTH, NestingMode};

async fn chain(r: &GroupRepo, n: usize, tag: &str) -> Vec<Group> {
    let mut v = vec![];
    for i in 0..n {
        v.push(r.create(&format!("{tag}{i}"), "").await.unwrap());
    }
    v
}

async fn link(r: &GroupRepo, child: &Group, parent: &Group) -> Result<(), Error> {
    let q = r
        .request_parent_in(child.id, parent.id, NestingMode::Single)
        .await?;
    r.approve_parent(q.id).await.map(|_| ())
}

#[tokio::test]
async fn sixteen_edges_are_allowed_the_seventeenth_is_not() {
    let r = repo().await;
    let g = chain(&r, MAX_NESTING_DEPTH + 2, "g").await;
    for i in 0..MAX_NESTING_DEPTH {
        link(&r, &g[i], &g[i + 1]).await.unwrap();
    }
    let e = link(&r, &g[MAX_NESTING_DEPTH], &g[MAX_NESTING_DEPTH + 1]).await;
    assert!(matches!(e, Err(Error::TooDeep { depth: 17, .. })), "{e:?}");
    // at the other end too
    let top = r.create("top", "").await.unwrap();
    let e = link(&r, &g[MAX_NESTING_DEPTH], &top).await;
    assert!(matches!(e, Err(Error::TooDeep { .. })), "{e:?}");
}

#[tokio::test]
async fn two_short_chains_may_not_be_joined_into_a_long_one() {
    let r = repo().await;
    let (a, b) = (chain(&r, 10, "a").await, chain(&r, 10, "b").await);
    for c in [&a, &b] {
        for i in 0..9 {
            link(&r, &c[i], &c[i + 1]).await.unwrap(); // 9 edges each
        }
    }
    // joining top of a under bottom of b would make 9 + 1 + 9 = 19
    let e = link(&r, &a[9], &b[0]).await;
    assert!(matches!(e, Err(Error::TooDeep { depth: 19, .. })), "{e:?}");
    // a diamond adds no depth
    let (x, y, z, w) = (
        r.create("x", "").await.unwrap(),
        r.create("y", "").await.unwrap(),
        r.create("z", "").await.unwrap(),
        r.create("w", "").await.unwrap(),
    );
    for (c, p) in [(&x, &y), (&x, &z), (&y, &w), (&z, &w)] {
        link(&r, c, p).await.unwrap();
    }
}
