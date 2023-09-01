use std::ops::Range;
use super::{
    error::{kind, Error},
    expr::Expr,
    literal::{Literal, LitSym},
    token::{CloseParen, OpenParen},
    Parse,
    Parser,
};

/// A parenthesized expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Paren {
    /// The inner expression.
    pub expr: Box<Expr>,

    /// The region of the source code that this literal was parsed from.
    pub span: Range<usize>,
}

impl Paren {
    /// Returns the span of the parenthesized expression.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }
}

impl<'source> Parse<'source> for Paren {
    fn std_parse(
        input: &mut Parser<'source>,
        recoverable_errors: &mut Vec<Error>
    ) -> Result<Self, Vec<Error>> {
        let open_paren = input.try_parse::<OpenParen>().forward_errors(recoverable_errors)?;
        let expr = match input.try_parse::<Expr>().forward_errors(recoverable_errors) {
            Ok(expr) => Ok(expr),
            Err(errs) => {
                if let Ok(close_paren) = input.try_parse::<CloseParen>().forward_errors(recoverable_errors) {
                    recoverable_errors.push(Error::new(
                        vec![open_paren.span.start..close_paren.span.end],
                        kind::EmptyParenthesis,
                    ));

                    let fake_expr = Expr::Literal(Literal::Symbol(LitSym {
                        name: String::new(),
                        span: 0..0,
                    }));

                    // return a fake expression for recovery purposes, and also so that we don't
                    // try to parse the close paren again below (which would add an extraneous
                    // error to the error list)
                    return Ok(Self {
                        expr: Box::new(fake_expr),
                        span: open_paren.span.start..close_paren.span.end,
                    });
                } else {
                    Err(errs)
                }
            },
        }?;
        let close_paren = input.try_parse::<CloseParen>()
            .forward_errors(recoverable_errors)
            .unwrap_or_else(|_| {
                recoverable_errors.push(Error::new(
                    vec![open_paren.span.clone()],
                    kind::UnclosedParenthesis { opening: true },
                ));

                // fake a close paren for recovery purposes
                CloseParen {
                    lexeme: "",
                    span: 0..0,
                }
            });
        Ok(Self {
            expr: Box::new(expr),
            span: open_paren.span.start..close_paren.span.end,
        })
    }
}
