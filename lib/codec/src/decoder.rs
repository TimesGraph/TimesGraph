use crate::then::Then;

pub trait Decoder: Sized {
    type Input;
    type Output;
    type Error;

    fn decode(self, input: &mut Self::Input) -> Then<Self, Self::Output, Self::Error>;
}
