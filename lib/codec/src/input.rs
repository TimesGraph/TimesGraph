use core::char;
use core::u32;
use core::usize;
use crate::step::{Step, In, Out, Over};

pub trait Input {
    type Token;

    fn head(&mut self) -> Step<Self::Token>;

    fn step(&mut self);

    fn over(&mut self);

    fn is_in(&mut self) -> bool {
        match self.head() {
            In(_) => true,
            _ => false,
        }
    }

    fn is_out(&mut self) -> bool {
        match self.head() {
            Out => true,
            _ => false,
        }
    }

    fn is_over(&mut self) -> bool {
        match self.head() {
            Over => true,
            _ => false,
        }
    }

    fn into_iter(self) -> InputIterator<Self> where Self: Sized {
        InputIterator { input: self }
    }
}

pub trait OffsetInput: Input {
    fn offset(&self) -> usize;
}

pub trait AsInput {
    type Token;

    type AsIn: Input<Token=Self::Token>;

    fn as_input(self) -> Self::AsIn;
}

impl<I> AsInput for I where I: Input {
    type Token = I::Token;

    type AsIn = I;

    fn as_input(self) -> I { self }
}

#[derive(Clone)]
pub struct InputIterator<I> {
    input: I,
}

impl<I> Iterator for InputIterator<I> where I: Input {
    type Item = I::Token;

    fn next(&mut self) -> Option<I::Token> {
        match self.input.head() {
            In(x) => {
                self.input.step();
                Some(x)
            },
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SliceInput<'a, T: 'a> {
    slice: &'a [T],
    offset: usize,
}

impl<'a, T: 'a + Clone> Input for SliceInput<'a, T> {
    type Token = T;

    fn head(&mut self) -> Step<T> {
        if self.offset < self.slice.len() {
            In(unsafe { self.slice.get_unchecked(self.offset).clone() })
        } else if self.offset < usize::MAX {
            Out
        } else {
            Over
        }
    }

    fn step(&mut self) {
        if self.offset < self.slice.len() {
            self.offset += 1;
        }
    }

    fn over(&mut self) {
        self.offset = usize::MAX;
    }
}

impl<'a, T: 'a + Clone> OffsetInput for SliceInput<'a, T> {
    fn offset(&self) -> usize {
        self.offset
    }
}

impl<'a, T: 'a + Clone> AsInput for &'a [T] {
    type Token = T;
    type AsIn = SliceInput<'a, T>;

    fn as_input(self) -> SliceInput<'a, T> {
        SliceInput {
            slice: self,
            offset: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Utf8Input<I: Input<Token=u8>> {
    input: I,
    head: u32,
    have: u8,
    b1: u8,
    b2: u8,
    b3: u8,
}

impl<I: Input<Token=u8>> Utf8Input<I> {
    pub const fn new(input: I) -> Self {
        Self {
            input: input,
            head: u32::MAX,
            have: 0,
            b1: 0,
            b2: 0,
            b3: 0,
        }
    }

    pub fn more(&mut self, input: I) {
        self.input = input;
    }

    #[inline]
    fn byte1(&mut self) -> Step<u32> {
        if self.have >= 1 {
            In(self.b1 as u32)
        } else {
            self.input.head().map(|c| {
                self.input.step();
                self.have = 1;
                self.b1 = c;
                c as u32
            })
        }
    }

    #[inline]
    fn byte2(&mut self) -> Step<u32> {
        if self.have >= 2 {
            In(self.b2 as u32)
        } else {
            self.input.head().map(|c| {
                self.input.step();
                self.have = 2;
                self.b2 = c;
                c as u32
            })
        }
    }

    #[inline]
    fn byte3(&mut self) -> Step<u32> {
        if self.have >= 3 {
            In(self.b3 as u32)
        } else {
            self.input.head().map(|c| {
                self.input.step();
                self.have = 3;
                self.b3 = c;
                c as u32
            })
        }
    }

    #[inline]
    fn byte4(&mut self) -> Step<u32> {
        debug_assert_eq!(self.have, 3);
        self.input.head().map(|c| {
            self.input.step();
            self.have = 4;
            c as u32
        })
    }

    #[inline]
    fn next(&mut self) -> Step<u32> {
        self.byte1().and_then(|b1| {
            self.next1(b1)
        })
    }

    #[inline]
    fn next1(&mut self, b1: u32) -> Step<u32> {
        if b1 <= 0x7F { // U+0000..U+007F
            In(b1)
        } else if b1 >= 0xC2 && b1 <= 0xF4 {
            self.byte2().and_then(|b2| {
                self.next2(b1, b2)
            })
        } else {
            In(0xFFFD)
        }
    }

    #[inline]
    fn next2(&mut self, b1: u32, b2: u32) -> Step<u32> {
        if b1 <= 0xDF {
            if b2 >= 0x80 && b2 <= 0xBF { // U+0080..U+07FF
                In((b1 & 0x1F) << 6 | b2 & 0x3F)
            } else {
                In(0xFFFD)
            }
        } else {
            self.byte3().and_then(|b3| {
                self.next3(b1, b2, b3)
            })
        }
    }

    #[inline]
    fn next3(&mut self, b1: u32, b2: u32, b3: u32) -> Step<u32> {
        if b1 == 0xE0 && b2 >= 0xA0 && b2 <= 0xBF ||
           b1 == 0xED && b2 >= 0x80 && b2 <= 0x9F ||
           b1 >= 0xE1 && b1 <= 0xEF && b2 >= 0x80 && b2 <= 0xBF {
            if b3 >= 0x80 && b3 <= 0xBF { // U+0800..U+FFFF
                In((b1 & 0x0F) << 12 | (b2 & 0x3F) << 6 | b3 & 0x3F)
            } else {
                In(0xFFFD)
            }
        } else {
            self.byte4().and_then(|b4| {
                self.next4(b1, b2, b3, b4)
            })
        }
    }

    #[inline]
    fn next4(&mut self, b1: u32, b2: u32, b3: u32, b4: u32) -> Step<u32> {
        if (b1 == 0xF0 && b2 >= 0x90 && b2 <= 0xBF ||
            b1 >= 0xF1 && b1 <= 0xF3 && b2 >= 0x80 && b2 <= 0xBF ||
            b1 == 0xF4 && b2 >= 0x80 && b2 <= 0x8F) &&
           b3 >= 0x80 && b3 <= 0xBF {
            if b4 >= 0x80 && b4 <= 0xBF { // U+10000..U+10FFFF
                In((b1 & 0x07) << 18 | (b2 & 0x3F) << 12 | (b3 & 0x3F) << 6 | b4 & 0x3F)
            } else {
                In(0xFFFD)
            }
        } else {
            In(0xFFFD)
        }
    }
}

impl<I: Input<Token=u8>> Input for Utf8Input<I> {
    type Token = char;

    fn head(&mut self) -> Step<char> {
        if self.head == u32::MAX {
            match self.next() {
                In(c) => {
                    self.head = c;
                },
                Out => return Out,
                Over => return Over,
            };
        }
        In(unsafe { char::from_u32_unchecked(self.head) })
    }

    fn step(&mut self) {
        self.head = u32::MAX;
        self.have = 0;
    }

    fn over(&mut self) {
        self.input.over();
    }
}

impl<I: OffsetInput<Token=u8>> OffsetInput for Utf8Input<I> {
    fn offset(&self) -> usize {
        self.input.offset() - self.have as usize
    }
}

pub type StrInput<'a> = Utf8Input<SliceInput<'a, u8>>;

impl<'a> AsInput for &'a str {
    type Token = char;
    type AsIn = StrInput<'a>;

    fn as_input(self) -> StrInput<'a> {
        Utf8Input::new(self.as_bytes().as_input())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_input() {
        let mut input = "test".as_input();
        assert_eq!(input.head(), In('t'));
        input.step();
        assert_eq!(input.head(), In('e'));
        input.step();
        assert_eq!(input.head(), In('s'));
        input.step();
        assert_eq!(input.head(), In('t'));
        input.step();
        assert_eq!(input.head(), Out);
    }

    #[test]
    fn test_utf8_input() {
        let mut input = "\0ÀÖØöø˿ͰͽͿ῿⁰↏Ⰰ⿯、퟿豈﷏ﷰ𐀀󯿿".as_input();
        assert_eq!(input.head(), In('\u{0}'));
        input.step();
        assert_eq!(input.head(), In('\u{C0}'));
        input.step();
        assert_eq!(input.head(), In('\u{D6}'));
        input.step();
        assert_eq!(input.head(), In('\u{D8}'));
        input.step();
        assert_eq!(input.head(), In('\u{F6}'));
        input.step();
        assert_eq!(input.head(), In('\u{F8}'));
        input.step();
        assert_eq!(input.head(), In('\u{2FF}'));
        input.step();
        assert_eq!(input.head(), In('\u{370}'));
        input.step();
        assert_eq!(input.head(), In('\u{37D}'));
        input.step();
        assert_eq!(input.head(), In('\u{37F}'));
        input.step();
        assert_eq!(input.head(), In('\u{1FFF}'));
        input.step();
        assert_eq!(input.head(), In('\u{2070}'));
        input.step();
        assert_eq!(input.head(), In('\u{218F}'));
        input.step();
        assert_eq!(input.head(), In('\u{2C00}'));
        input.step();
        assert_eq!(input.head(), In('\u{2FEF}'));
        input.step();
        assert_eq!(input.head(), In('\u{3001}'));
        input.step();
        assert_eq!(input.head(), In('\u{D7FF}'));
        input.step();
        assert_eq!(input.head(), In('\u{F900}'));
        input.step();
        assert_eq!(input.head(), In('\u{FDCF}'));
        input.step();
        assert_eq!(input.head(), In('\u{FDF0}'));
        input.step();
        assert_eq!(input.head(), In('\u{10000}'));
        input.step();
        assert_eq!(input.head(), In('\u{EFFFF}'));
        input.step();
        assert_eq!(input.head(), Out);
    }
}
