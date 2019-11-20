// Dependencies

use std::ops::Add;

// Enums

#[derive(Debug, PartialEq)]
pub enum Size {
    Fixed(usize),
    Variable
}

// Operators

impl Add<Size> for Size {
    type Output = Size;

    fn add(self, rhs: Size) -> Size {
        if let (Size::Fixed(left), Size::Fixed(right)) = (self, rhs) {
            Size::Fixed(left + right)
        } else {
            Size::Variable
        }
    }
}

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(Size::Fixed(4) + Size::Fixed(5), Size::Fixed(9));
        assert_eq!(Size::Fixed(4) + Size::Variable, Size::Variable);
        assert_eq!(Size::Variable + Size::Fixed(5), Size::Variable);
        assert_eq!(Size::Variable + Size::Variable, Size::Variable);
    }
}
