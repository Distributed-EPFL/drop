use crate::crypto::hash::{Digest, SIZE};

use std::ops::Index;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Direction {
    Left,
    Right
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub(super) struct Path([u8; SIZE]);

#[derive(Debug, Clone, Copy, Eq)]
pub(super) struct Prefix {
    path: Path,
    depth: u8
}

impl Index<u8> for Path {
    type Output = Direction;

    fn index(&self, index: u8) -> &Self::Output {
        let (byte, bit) = split(index);
        let mask = 1 << (7 - bit);

        if self.0[byte] & mask != 0 { &Direction::Left } else { &Direction::Right }
    }
}

impl Index<u8> for Prefix {
    type Output = Direction;

    fn index(&self, index: u8) -> &Self::Output {
        debug_assert!(index < self.depth);
        &self.path[index]
    }
}

impl PartialEq for Prefix {
    fn eq(&self, rho: &Self) -> bool {
        self.depth == rho.depth && deepeq(&self.path, &rho.path, self.depth)
    }
}

impl Path {
    pub fn empty() -> Self {
        Path([0; SIZE])
    }

    pub fn new(digest: Digest) -> Self {
        Path(digest.0)
    }

    pub fn set(&mut self, index: u8, value: Direction) {
        let (byte, bit) = split(index);
        
        if value == Direction::Left {
            self.0[byte] |= 1 << (7 - bit);
        } else {
            self.0[byte] &= !(1 << (7 - bit));
        }
    }
}

impl Prefix {
    pub fn new(path: Path, depth: u8) -> Self {
        Prefix{path, depth}
    }

    pub fn root() -> Self {
        Prefix{path: Path::empty(), depth: 0}
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }

    pub fn left(&self) -> Self {
        self.child(Direction::Left)
    }

    pub fn right(&self) -> Self {
        self.child(Direction::Right)
    }

    fn child(&self, direction: Direction) -> Self {
        let mut path = self.path;
        path.set(self.depth, direction);

        Prefix{path, depth: self.depth + 1}
    }

    pub fn contains(&self, path: &Path) -> bool {
        deepeq(&self.path, path, self.depth)
    }
}

fn split(index: u8) -> (usize, u8) {
    ((index / 8) as usize, index % 8)
}

fn deepeq(lho: &Path, rho: &Path, depth: u8) -> bool {
    let (full, overflow) = split(depth);
        
    if lho.0[0..full] != rho.0[0..full] {
        return false;
    }

    if overflow > 0 {
        let shift = 8 - overflow;
        (lho.0[full] >> shift) == (rho.0[full] >> shift)
    } else { true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hash;
    use std::iter;
    use std::vec::Vec;

    fn path_from_directions(directions: &Vec<Direction>) -> Path {
        let mut path = Path::empty();

        for index in 0..directions.len() {
            path.set(index as u8, directions[index]);
        }

        path
    }

    fn directions_from_path(path: &Path, until: u8) -> Vec<Direction> {
        (0..until).map(|index| path[index]).collect()
    }

    fn directions_from_prefix(prefix: &Prefix) -> Vec<Direction> {
        directions_from_path(&prefix.path, prefix.depth())
    }

    #[test]
    fn path() {
        let reference = vec![Direction::Right, Direction::Right, Direction::Right, Direction::Left, 
                             Direction::Right, Direction::Right, Direction::Right, Direction::Left, 
                             Direction::Left, Direction::Left, Direction::Right, Direction::Left,
                             Direction::Left, Direction::Right, Direction::Left, Direction::Right];

        assert_eq!(directions_from_path(&Path::empty(), (8 * SIZE - 1) as u8), iter::repeat(Direction::Right).take(8 * SIZE - 1).collect::<Vec<Direction>>());
        assert_eq!(directions_from_path(&Path::new(hash(&0u32).unwrap()), reference.len() as u8), reference);
        assert_eq!(directions_from_path(&path_from_directions(&reference), reference.len() as u8), reference);
    }

    #[test]
    fn ordering() {
        assert!(&path_from_directions(&vec![Direction::Right]) < &path_from_directions(&vec![Direction::Left]));
        assert!(&path_from_directions(&vec![Direction::Right]) < &path_from_directions(&vec![Direction::Right, Direction::Left]));
        assert!(&path_from_directions(&vec![Direction::Left, Direction::Right, Direction::Left]) < &path_from_directions(&vec![Direction::Left, Direction::Left, Direction::Left, Direction::Left, Direction::Left]));

        let lesser = vec![Direction::Right, Direction::Right, Direction::Right, Direction::Left, 
                          Direction::Right, Direction::Right, Direction::Right, Direction::Left, 
                          Direction::Left, Direction::Left, Direction::Right, Direction::Left,
                          Direction::Left, Direction::Right, Direction::Left, Direction::Right];

        let mut greater = lesser.clone();
        greater.push(Direction::Left);

        assert!(&path_from_directions(&lesser) < &path_from_directions(&greater));
    }

    #[test]
    fn prefix() {
        let reference = vec![Direction::Right, Direction::Right, Direction::Right, Direction::Left, 
                             Direction::Right, Direction::Right, Direction::Right, Direction::Left, 
                             Direction::Left, Direction::Left, Direction::Right, Direction::Left,
                             Direction::Left, Direction::Right, Direction::Left, Direction::Right];

        let path = path_from_directions(&reference);

        assert_eq!(directions_from_prefix(&Prefix::new(path, reference.len() as u8)), reference);
        assert_eq!(directions_from_prefix(&Prefix::root()), vec![]);
        assert_eq!(directions_from_prefix(&Prefix::root().right().right().right().left().right().right().right().left().left().left().right().left().left().right().left().right()), reference);

        assert!(Prefix::root().contains(&path_from_directions(&vec![Direction::Left])));
        assert!(Prefix::root().contains(&path_from_directions(&vec![Direction::Right])));

        assert!(Prefix::root().right().contains(&path));
        assert!(!Prefix::root().left().contains(&path));
        
        assert!(Prefix::root().right().right().right().left().right().right().right().contains(&path));
        assert!(!Prefix::root().right().right().right().left().right().right().left().contains(&path));

        assert!(Prefix::new(path, reference.len() as u8).contains(&path));
        assert!(Prefix::new(path, reference.len() as u8).right().contains(&path));
        assert!(!Prefix::new(path, reference.len() as u8).left().contains(&path));

        assert_eq!(Prefix::root(), Prefix::root());
        assert_eq!(Prefix::root().left(), Prefix::root().left());
        assert_ne!(Prefix::root().left(), Prefix::root().right());
        assert_ne!(Prefix::root(), Prefix::root().left());
        assert_eq!(Prefix::root().right().right().right().left().right().right().right(), Prefix::root().right().right().right().left().right().right().right());
        assert_ne!(Prefix::root().right().right().right().left().right().right().right(), Prefix::root().right().right().right().left().right().right().left());
        assert_ne!(Prefix::root().right().right().right().left().right().right().right(), Prefix::root().right().right().right().left().right().right());
    }
}