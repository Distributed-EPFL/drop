use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::collections::VecDeque;
use std::vec::Vec;

pub trait Typename {
    fn typename() -> String;
}

macro_rules! implement {
    ($($type:ty), *) => ($(
        impl Typename for $type {
            fn typename() -> String {
                stringify!($type).to_string()
            }
        }
    )*);
}

implement!(
    i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize, f32, f64,
    str, String
);

macro_rules! implement {
    ($($size:expr), *) => ($(
        impl<Type: Typename> Typename for [Type; $size] {
            fn typename() -> String {
                format!("[{}; {}]", Type::typename(), $size)
            }
        }
    )*);
}

implement!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
    22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 64, 128, 256, 512, 1024, 2048,
    4096, 8192
);

macro_rules! implement {
    ($($types:ident),+) => {
        impl<$($types: Typename),+> Typename for ($($types,)+) {
            fn typename() -> String {
                format!("({})", vec![$($types::typename()),+].join(", "))
            }
        }
    }
}

implement!(A);
implement!(A, B);
implement!(A, B, C);
implement!(A, B, C, D);
implement!(A, B, C, D, E);
implement!(A, B, C, D, E, F);
implement!(A, B, C, D, E, F, G);
implement!(A, B, C, D, E, F, G, H);
implement!(A, B, C, D, E, F, G, H, I);
implement!(A, B, C, D, E, F, G, H, I, J);
implement!(A, B, C, D, E, F, G, H, I, J, K);
implement!(A, B, C, D, E, F, G, H, I, J, K, L);

impl<Type: Typename> Typename for &'static Type {
    fn typename() -> String {
        format!("&'static {}", Type::typename())
    }
}

// Collections

impl<Item: Typename> Typename for Vec<Item> {
    fn typename() -> String {
        format!("Vec<{}>", Item::typename())
    }
}

impl<Item: Typename> Typename for BinaryHeap<Item> {
    fn typename() -> String {
        format!("BinaryHeap<{}>", Item::typename())
    }
}

impl<Key: Typename, Value: Typename> Typename for BTreeMap<Key, Value> {
    fn typename() -> String {
        format!("BTreeMap<{}, {}>", Key::typename(), Value::typename())
    }
}

impl<Item: Typename> Typename for BTreeSet<Item> {
    fn typename() -> String {
        format!("BTreeSet<{}>", Item::typename())
    }
}

impl<Key: Typename, Value: Typename> Typename for HashMap<Key, Value> {
    fn typename() -> String {
        format!("HashMap<{}, {}>", Key::typename(), Value::typename())
    }
}

impl<Item: Typename> Typename for HashSet<Item> {
    fn typename() -> String {
        format!("HashSet<{}>", Item::typename())
    }
}

impl<Item: Typename> Typename for LinkedList<Item> {
    fn typename() -> String {
        format!("LinkedList<{}>", Item::typename())
    }
}

impl<Item: Typename> Typename for VecDeque<Item> {
    fn typename() -> String {
        format!("VecDeque<{}>", Item::typename())
    }
}
