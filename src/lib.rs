use std::ops::Range;

mod char_reader;
mod compiler;
mod errors;
pub mod tokenizer;

pub(crate) use char_reader::*;
pub use compiler::compile;
pub use errors::*;
pub use tokenizer::tokenize;

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq)]
pub struct Span {
    pub pos: Range<usize>,
    pub line: Range<usize>,
    pub col: Range<usize>,
}

impl Span {
    pub fn slice<'a>(&self, contents: &'a str) -> &'a str {
        &contents[self.pos.clone()]
    }
}
