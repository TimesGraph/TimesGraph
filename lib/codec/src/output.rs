use core::str;

pub trait Output {
    type Token;

    type Out;

    type Err;

    fn is_full(&self) -> bool;

    fn push(&mut self, token: Self::Token);

    fn take_out(self) -> Result<Self::Out, Self::Err>;
}

pub trait IntoOutput {
    type Token;

    type IntoOut;

    fn into_output(self) -> Self::IntoOut;
}

impl<O> IntoOutput for O where O: Output {
    type Token = O::Token;
    type IntoOut = O;

    fn into_output(self) -> O { self }
}

#[derive(PartialEq, Eq, Debug)]
pub struct SliceOutput<'a, T: 'a> {
    slice: &'a mut [T],
    offset: usize,
}

impl<'a, T: 'a> SliceOutput<'a, T> {
    #[inline]
    pub fn new(slice: &'a mut [T]) -> Self {
        SliceOutput {
            slice: slice,
            offset: 0,
        }
    }
}

impl<'a, T: 'a> Output for SliceOutput<'a, T> {
    type Token = T;
    type Out = &'a mut [T];
    type Err = ();

    fn is_full(&self) -> bool {
        self.offset >= self.slice.len()
    }

    fn push(&mut self, token: T) {
        self.slice[self.offset] = token;
        self.offset += 1;
    }

    fn take_out(self) -> Result<&'a mut [T], ()> {
        Ok(&mut self.slice[..self.offset])
    }
}

impl<'a, T: 'a> IntoOutput for &'a mut [T] {
    type Token = T;
    type IntoOut = SliceOutput<'a, T>;

    #[inline]
    fn into_output(self) -> SliceOutput<'a, T> {
        SliceOutput::new(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Utf8Output<O: Output<Token=u8>> {
    output: O,
    have: u8,
    b1: u8,
    b2: u8,
    b3: u8,
}

impl<O: Output<Token=u8>> Utf8Output<O> {
    pub const fn new(output: O) -> Self {
        Self {
            output: output,
            have: 0,
            b1: 0,
            b2: 0,
            b3: 0,
        }
    }

    fn push_byte(&mut self, b: u8) {
        if !self.output.is_full() {
            self.output.push(b);
        } else if self.have == 0 {
            self.b1 = b;
            self.have = 1;
        } else if self.have == 1 {
            self.b2 = b;
            self.have = 2;
        } else if self.have == 2 {
            self.b3 = b;
            self.have = 3;
        } else {
            unreachable!();
        }
    }

    fn flush(&mut self) {
        // let self.output panic if full
        loop {
            if self.have == 0 {
                return;
            } else if self.have == 1 {
                self.output.push(self.b1);
                self.have = 0;
                self.b1 = 0;
                return;
            } else if self.have == 2 {
                self.output.push(self.b2);
                self.have = 1;
                self.b2 = 0;
            } else if self.have == 3 {
                self.output.push(self.b3);
                self.have = 2;
                self.b3 = 0;
            } else {
                unreachable!();
            }
        }
    }
}

impl<O: Output<Token=u8>> Output for Utf8Output<O> {
    type Token = char;
    type Out = O::Out;
    type Err = O::Err;

    fn is_full(&self) -> bool {
        self.output.is_full()
    }

    fn push(&mut self, c: char) {
        self.flush();
        let c = c as u32;
        if c <= 0x7F { // U+0000..U+007F
            self.push_byte(c as u8);
        } else if c >= 0x80 && c <= 0x7FF { // U+0080..U+07FF
            self.push_byte((0xC0 | c >> 6) as u8);
            self.push_byte((0x80 | c & 0x3F) as u8);
        } else if c >= 0x0800 && c <= 0xFFFF || // U+0800..U+D7FF
                  c >= 0xE000 && c <= 0xFFFF { // U+E000..U+FFFF
            self.push_byte((0xE0 | c >> 12) as u8);
            self.push_byte((0x80 | c >> 6 & 0x3F) as u8);
            self.push_byte((0x80 | c & 0x3F) as u8);
        } else if c >= 0x10000 && c <= 0x10FFFF { // U+10000..U+10FFFF
            self.push_byte((0xF0 | c >> 18) as u8);
            self.push_byte((0x80 | c >> 12 & 0x3F) as u8);
            self.push_byte((0x80 | c >> 6 & 0x3F) as u8);
            self.push_byte((0x80 | c & 0x3F) as u8);
        } else { // surrogate or invalid code point
            self.push_byte(0xEF);
            self.push_byte(0xBF);
            self.push_byte(0xBD);
        }
    }

    fn take_out(self) -> Result<O::Out, O::Err> {
        self.output.take_out()
    }
}


pub struct StrOutput<'a> {
    output: Utf8Output<SliceOutput<'a, u8>>,
}

impl<'a> StrOutput<'a> {
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self {
            output: Utf8Output::new(slice.into_output())
        }
    }
}

impl<'a> Output for StrOutput<'a> {
    type Token = char;
    type Out = &'a str;
    type Err = ();

    #[inline]
    fn is_full(&self) -> bool {
        self.output.is_full()
    }

    #[inline]
    fn push(&mut self, c: char) {
        self.output.push(c);
    }

    fn take_out(self) -> Result<&'a str, ()> {
        match self.output.take_out() {
            Ok(slice) => Ok(unsafe { str::from_utf8_unchecked(slice) }),
            Err(_) => Err(()),
        }
    }
}
