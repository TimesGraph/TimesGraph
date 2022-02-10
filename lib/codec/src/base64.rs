use core::fmt;
use core::marker::PhantomData;
use core::mem;
use crate::step::{In, Out, Over};
use crate::then::{Then, Cont, Done, Fail};
use crate::input::{Input, AsInput};
use crate::output::{Output, IntoOutput};
use crate::decoder::Decoder;
use crate::encoder::Encoder;
///
/// Base64 is used for encoding and decoding small image file
/// 
pub trait DecodeBase64: Sized {
    fn decode_base64_input<I>(input: &mut I) -> Result<Self, Base64Error> where I: Input<Token=char>;

    fn decode_base64(string: &str) -> Result<Self, Base64Error> {
        Self::decode_base64_input(&mut string.as_input())
    }
}

pub trait EncodeBase64 {
    fn encode_base64_output<O>(&self, output: O, alphabet: Base64Alphabet)
        -> Result<O::Out, O::Err> where O: Output<Token=char>;

    fn encode_base64<I, O>(&self, output: I) -> O::Out
        where I: IntoOutput<IntoOut=O>, O: Output<Token=char>, O::Err: fmt::Debug {
        let output = output.into_output();
        self.encode_base64_output(output, Base64).unwrap()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Base64Alphabet {
    Base64,
    Base64Url,
}
pub use self::Base64Alphabet::{Base64, Base64Url};

impl Base64Alphabet {
    pub fn as_str(self) -> &'static [u8; 64] {
        let alphabet = match self {
            Base64    => "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
            Base64Url => "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
        };
        unsafe { mem::transmute(alphabet.as_ptr()) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Base64Error {
    Unexpected,
    Unpadded,
}

pub struct Base64Decoder<I: Input<Token=char>, O: Output<Token=u8>> {
    pub output: O,
    p: u8,
    q: u8,
    r: u8,
    padded: bool,
    state: u32,
    input: PhantomData<I>,
}

pub struct Base64Encoder<I: Input<Token=u8>, O: Output<Token=char>> {
    alphabet: &'static [u8; 64],
    pub input: I,
    x: u8,
    y: u8,
    z: u8,
    padded: bool,
    state: u32,
    output: PhantomData<O>,
}

impl<I, O> Base64Decoder<I, O> where I: Input<Token=char>, O: Output<Token=u8> {
    pub fn new(output: O) -> Self {
        Self {
            output: output,
            p: 0,
            q: 0,
            r: 0,
            padded: true,
            state: 1,
            input: PhantomData,
        }
    }

    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
        self
    }

    pub fn consume(mut self, input: &mut I) -> Result<O::Out, Base64Error> where O::Err: fmt::Debug {
        loop {
            match self.decode(input) {
                Done(output) => return Ok(output),
                Fail(error) => return Err(error),
                Cont(next) => {
                    if input.is_out() {
                        input.over();
                        self = next;
                    } else {
                        return Err(Base64Error::Unexpected);
                    }
                },
            }
        }
    }
}

impl<I, O> Decoder for Base64Decoder<I, O>
    where I: Input<Token=char>,
          O: Output<Token=u8>,
          O::Err: fmt::Debug {

    type Input = I;
    type Output = O::Out;
    type Error = Base64Error;

    fn decode(mut self, input: &mut I) -> Then<Self, O::Out, Base64Error> {
        loop {
            match self.state {
                1 => {
                    match input.head() {
                        In(c) if is_base64_char(c) => {
                            input.step();
                            self.p = decode_base64_char(c);
                            self.state = 2;
                        },
                        In(_) | Over => return Done(self.output.take_out().unwrap()),
                        Out => return Cont(self),
                    };
                },
                2 => {
                    match input.head() {
                        In(c) if is_base64_char(c) => {
                            input.step();
                            self.q = decode_base64_char(c);
                            self.state = 3;
                        },
                        In(_) | Over => return Fail(Base64Error::Unexpected),
                        Out => return Cont(self),
                    };
                },
                3 => {
                    match input.head() {
                        In(c) if is_base64_char(c) || c == '=' => {
                            input.step();
                            self.r = decode_base64_char(c);
                            if c != '=' {
                                self.state = 4;
                            } else {
                                self.state = 5;
                            }
                        },
                        In(_) | Over if !self.padded => {
                            decode_base64_quantum(self.p, self.q, 255, 255, &mut self.output);
                            return Done(self.output.take_out().unwrap());
                        },
                        In(_) | Over => return Fail(Base64Error::Unpadded),
                        Out => return Cont(self),
                    };
                },
                4 => {
                    match input.head() {
                        In(c) if is_base64_char(c) || c == '=' => {
                            input.step();
                            let s = decode_base64_char(c);
                            decode_base64_quantum(self.p, self.q, self.r, s, &mut self.output);
                            self.r = 0;
                            self.q = 0;
                            self.p = 0;
                            if c != '=' {
                                self.state = 1;
                            } else {
                                return Done(self.output.take_out().unwrap());
                            }
                        },
                        In(_) | Over if !self.padded => {
                            decode_base64_quantum(self.p, self.q, self.r, 255, &mut self.output);
                            return Done(self.output.take_out().unwrap());
                        }
                        In(_) | Over => return Fail(Base64Error::Unpadded),
                        Out => return Cont(self),
                    };
                },
                5 => {
                    match input.head() {
                        In('=') => {
                            input.step();
                            decode_base64_quantum(self.p, self.q, self.r, 255, &mut self.output);
                            self.r = 0;
                            self.q = 0;
                            self.p = 0;
                            return Done(self.output.take_out().unwrap());
                        },
                        In(_) | Over => return Fail(Base64Error::Unpadded),
                        Out => return Cont(self),
                    }
                },
                _ => unreachable!(),
            };
        }
    }
}

impl<I, O> Base64Encoder<I, O> where I: Input<Token=u8>, O: Output<Token=char> {
    pub fn new(input: I, alphabet: Base64Alphabet) -> Self {
        Self {
            alphabet: alphabet.as_str(),
            input: input,
            x: 0,
            y: 0,
            z: 0,
            padded: true,
            state: 1,
            output: PhantomData,
        }
    }

    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
        self
    }

    pub fn produce(mut self, mut output: O) -> Result<O::Out, O::Err> {
        loop {
            match self.encode(&mut output) {
                Done(_) => return output.take_out(),
                Fail(_) => unreachable!(),
                Cont(next) => {
                    self = next;
                    self.input.over();
                }
            }
        }
    }

    fn encode_base64_digit(&self, x: u8) -> char {
        debug_assert!(x < 64);
        unsafe { (*self.alphabet.get_unchecked(x as usize)) as char }
    }
}

impl<I, O> Encoder for Base64Encoder<I, O> where I: Input<Token=u8>, O: Output<Token=char> {
    type Input = I;
    type Output = O;
    type Error = ();

    fn encode(mut self, output: &mut O) -> Then<Self, I, ()> {
        while !output.is_full() {
            match self.state {
                1 => {
                    match self.input.head() {
                        In(x) => {
                            self.input.step();
                            self.x = x;
                            self.state = 2;
                            output.push(self.encode_base64_digit(x >> 2));
                        },
                        Over => return Done(self.input),
                        Out => break,
                    };
                },
                2 => {
                    match self.input.head() {
                        In(y) => {
                            self.input.step();
                            self.y = y;
                            self.state = 3;
                            output.push(self.encode_base64_digit((self.x << 4 | y >> 4) & 0x3F));
                        },
                        Over => {
                            output.push(self.encode_base64_digit(self.x << 4 & 0x3F));
                            if self.padded {
                                self.state = 5;
                            } else {
                                return Done(self.input);
                            }
                        },
                        Out => break,
                    };
                },
                3 => {
                    match self.input.head() {
                        In(z) => {
                            self.input.step();
                            self.z = z;
                            self.state = 4;
                            output.push(self.encode_base64_digit((self.y << 2 | z >> 6) & 0x3F));
                        },
                        Over => {
                            output.push(self.encode_base64_digit(self.y << 2 & 0x3F));
                            if self.padded {
                                self.state = 6;
                            } else {
                                return Done(self.input);
                            }
                        },
                        Out => break,
                    }
                },
                4 => {
                    output.push(self.encode_base64_digit(self.z & 0x3F));
                    self.z = 0;
                    self.y = 0;
                    self.x = 0;
                    self.state = 1;
                },
                5 => {
                    output.push('=');
                    self.state = 6;
                },
                6 => {
                    output.push('=');
                    return Done(self.input);
                },
                _ => unreachable!(),
            };
        }
        return Cont(self);
    }
}

#[inline]
fn is_base64_char(c: char) -> bool {
    c >= '0' && c <= '9' ||
    c >= 'A' && c <= 'Z' ||
    c >= 'a' && c <= 'z' ||
    c == '+' || c == '-' ||
    c == '/' || c == '_'
}

#[inline]
fn decode_base64_char(c: char) -> u8 {
    if c >= 'A' && c <= 'Z' {
        c as u8 - 'A' as u8
    } else if c >= 'a' && c <= 'z' {
        26 + (c as u8 - 'a' as u8)
    } else if c >= '0' && c <= '9' {
        52 + (c as u8 - '0' as u8)
    } else if c == '+' || c == '-' {
        62
    } else if c == '/' || c == '_' {
        63
    } else if c == '=' {
        255
    } else {
      unreachable!()
    }
}

fn decode_base64_quantum<O>(p: u8, q: u8, r: u8, s: u8, output: &mut O)
    where O: Output<Token=u8> {
    if r < 64 {
        if s < 64 {
            output.push(p << 2 | q >> 4);
            output.push(q << 4 | r >> 2);
            output.push(r << 6 | s);
        } else {
            output.push(p << 2 | q >> 4);
            output.push(q << 4 | r >> 2);
        }
    } else {
        debug_assert_eq!(s, 255);
        output.push((p << 2) | (q >> 4));
    }
}

#[cfg(test)]
mod tests {
    use crate::output::{SliceOutput, StrOutput};
    use super::*;

    fn assert_transcodes(encoded: &str, decoded: &[u8]) {
        let mut buffer = [0u8; 1024];
        let decoder = Base64Decoder::new(SliceOutput::new(&mut buffer));
        assert_eq!(decoder.consume(&mut encoded.as_input()).unwrap(), decoded);
        let mut buffer = [0u8; 1024];
        let encoder = Base64Encoder::new(decoded.as_input(), Base64);
        assert_eq!(encoder.produce(StrOutput::new(&mut buffer)).unwrap(), encoded);
    }

    #[test]
    fn test_base64_transcode() {
        assert_transcodes("AA==", &[0]);
        assert_transcodes("AAA=", &[0, 0]);
        assert_transcodes("AAAA", &[0, 0, 0]);
        assert_transcodes("+w==", &[251]);
        assert_transcodes("++8=", &[251, 239]);
        assert_transcodes("++++", &[251, 239, 190]);
        assert_transcodes("ABCDabcd12/+", &[0, 16, 131, 105, 183, 29, 215, 111, 254]);
        assert_transcodes("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789/+",
                          &[0, 16, 131, 16, 81, 135, 32, 146, 139, 48, 211, 143, 65, 20,
                            147, 81, 85, 151, 97, 150, 155, 113, 215, 159, 130, 24, 163,
                            146, 89, 167, 162, 154, 171, 178, 219, 175, 195, 28, 179, 211,
                            93, 183, 227, 158, 187, 243, 223, 254]);
    }
}
