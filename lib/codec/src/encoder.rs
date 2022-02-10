use crate::then::Then;

pub trait Encoder: Sized {
    type Input;
    type Output;
    type Error;

    fn encode(self, output: &mut Self::Output) -> Then<Self, Self::Input, Self::Error>;
}
