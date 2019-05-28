// Structs

#[derive(Debug, PartialEq)]
pub struct Size(usize);

// Implementations

impl Size {
    pub const fn fixed(size: usize) -> Size {
        Size(size)
    }

    pub const fn variable() -> Size {
        Size(0)
    }

    pub const fn add(lho: Size, rho: Size) -> Size {
        // As of May 28, 2019, Rust does not provide support for `if` or `match` inside a
        // `const fn`. What follows is an implementation of `if` using the absorbing property of
        // the multiplication by zero. The second value of `fixed` is zero if either element of the
        // first `fixed` tuple is false. The array access in the last line simulates a ternary
        // operator.

        let fixed = (lho.0 != 0, rho.0 != 0);
        let fixed = (fixed.0 as usize) * (fixed.1 as usize);
        Size([0, lho.0 + rho.0][fixed])
    }

    pub const fn mul(lho: usize, rho: Size) -> Size {
        Size(lho * rho.0)
    }

    pub fn is_fixed(&self) -> bool {
        self.0 != 0
    }

    pub fn is_variable(&self) -> bool {
        self.0 == 0
    }

    pub fn size(&self) -> usize {
        assert!(self.0 != 0);
        self.0
    }
}

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods() {
        assert!(Size::fixed(4).is_fixed());
        assert!(Size::variable().is_variable());
        assert_eq!(Size::fixed(4).size(), 4);
    }

    #[test]
    fn add() {
        assert_eq!(Size::add(Size::fixed(4), Size::fixed(5)), Size::fixed(9));
        assert!(Size::add(Size::fixed(4), Size::variable()).is_variable());
        assert!(Size::add(Size::variable(), Size::fixed(5)).is_variable());
        assert!(Size::add(Size::variable(), Size::variable()).is_variable());
    }

    #[test]
    fn mul() {
        assert_eq!(Size::mul(2, Size::fixed(5)), Size::fixed(10));
        assert!(Size::mul(2, Size::variable()).is_variable());
    }

    #[test]
    #[should_panic]
    fn bounds() {
        Size::variable().size();
    }
}
