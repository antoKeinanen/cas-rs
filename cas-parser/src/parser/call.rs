use std::ops::Range;
use crate::{
    parser::{
        error::Error,
        expr::Expr,
        literal::LitSym,
        token::{CloseParen, OpenParen},
        Parse,
        Parser,
    },
    tokenizer::TokenKind,
};

/// A function call, such as `func(x, -40)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The name of the function to call.
    pub name: LitSym,

    /// The arguments to the function.
    pub args: Vec<Expr>,

    /// The region of the source code that this function call was parsed from.
    pub span: Range<usize>,

    /// The span of the parentheses that surround the arguments.
    pub paren_span: Range<usize>,
}

impl Call {
    /// Returns the span of the function call.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Returns a set of two spans, where the first is the span of the function name (with the
    /// opening parenthesis) and the second is the span of the closing parenthesis.
    pub fn outer_span(&self) -> [Range<usize>; 2] {
        [
            self.name.span.start..self.paren_span.start + 1,
            self.paren_span.end - 1..self.paren_span.end,
        ]
    }
}

impl Parse for Call {
    fn parse(input: &mut Parser) -> Result<Self, Error> {
        let name = input.try_parse::<LitSym>()?;
        let open_paren = input.try_parse::<OpenParen>()?;
        let args = input.try_parse_delimited::<Expr>(TokenKind::Comma)?;
        let close_paren = input.try_parse::<CloseParen>()?;

        // use `name` here before it is moved into the struct
        let span = name.span.start..close_paren.span.end;
        Ok(Self {
            name,
            args,
            span,
            paren_span: open_paren.span.start..close_paren.span.end,
        })
    }
}
