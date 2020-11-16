use std::hash::Hash;

use classic_derive::message;

use serde::{Deserialize, Serialize};

pub trait Message: Clone + PartialEq + Eq + Hash {}

#[test]
fn eq() {
    #[message]
    struct T(u8, u16, u32);

    let t1 = T(0, 1, 2);
    let t2 = T(0, 1, 2);

    assert_eq!(t1, t2);
}

#[test]
fn generics() {
    #[message]
    struct Gen<T> {
        other: T,
    }

    let g1 = Gen { other: 182usize };
    let g2 = Gen { other: 182usize };

    assert_eq!(g1, g2);
}

#[test]
fn bounds() {
    #[message]
    struct R<T> {
        u: T,
    }
}
