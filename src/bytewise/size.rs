// Dependencies

use std::ops::Add;

// Enums

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
