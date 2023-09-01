use std::ops::Range;
use crate::{
    parser::{
        error::{kind::InvalidAssignmentLhs, Error},
        expr::Expr,
        literal::{Literal, LitSym},
        token::{Assign as AssignOp, CloseParen, OpenParen},
        Parse,
        Parser,
        ParseResult,
    },
    tokenizer::TokenKind,
    return_if_ok,
};

/// A parameter of a function declaration, such as `x` or `y = 1` in the declaration `f(x, y = 1) =
/// x^y`.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    /// A parameter with no default value, such as `x` in `f(x) = x^2`.
    Symbol(LitSym),

    /// A parameter with a default value, such as `y = 1` in `f(x, y = 1) = x^y`.
    Default(LitSym, Expr),
}

impl Param {
    /// Returns the symbol of the parameter.
    pub fn symbol(&self) -> &LitSym {
        match self {
            Param::Symbol(symbol) => symbol,
            Param::Default(symbol, _) => symbol,
        }
    }
}

impl<'source> Parse<'source> for Param {
    fn std_parse(
        input: &mut Parser<'source>,
        recoverable_errors: &mut Vec<Error>
    ) -> Result<Self, Vec<Error>> {
        let symbol = input.try_parse::<LitSym>().forward_errors(recoverable_errors)?;

        if input.try_parse::<AssignOp>().is_ok() {
            let default = input.try_parse::<Expr>().forward_errors(recoverable_errors)?;
            Ok(Param::Default(symbol, default))
        } else {
            Ok(Param::Symbol(symbol))
        }
    }
}

/// A function header, **not including the body**. Functions can have multiple parameters with
/// optional default values, like in `f(x, y = 1)`. When a function with this header is called, the
/// default values are used (i.e. `y = 1`), unless the caller provides their own values (`f(2,
/// 3)`).
#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    /// The name of the function.
    pub name: LitSym,

    /// The parameters of the function.
    pub params: Vec<Param>,

    /// The region of the source code that this function header was parsed from.
    pub span: Range<usize>,
}

impl FuncHeader {
    /// Returns the span of the function header.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }
}

impl<'source> Parse<'source> for FuncHeader {
    fn std_parse(
        input: &mut Parser<'source>,
        recoverable_errors: &mut Vec<Error>
    ) -> Result<Self, Vec<Error>> {
        let name = input.try_parse::<LitSym>().forward_errors(recoverable_errors)?;
        input.try_parse::<OpenParen>().forward_errors(recoverable_errors)?;
        let params = input.try_parse_delimited::<Param>(TokenKind::Comma).forward_errors(recoverable_errors)?;
        let close_paren = input.try_parse::<CloseParen>().forward_errors(recoverable_errors)?;

        let span = name.span.start..close_paren.span.end;
        Ok(Self { name, params, span })
    }
}

/// An assignment target, such as `x` or `f(x)`.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    /// A symbol, such as `x`.
    Symbol(LitSym),

    /// A function, such as `f(x)`.
    Func(FuncHeader),
}

impl AssignTarget {
    /// Returns the span of the assignment target.
    pub fn span(&self) -> Range<usize> {
        match self {
            AssignTarget::Symbol(symbol) => symbol.span.clone(),
            AssignTarget::Func(func) => func.span(),
        }
    }

    /// Tries to convert a general [`Expr`] into an [`AssignTarget`]. This is used when parsing
    /// assignment expressions, such as `x = 1` or `f(x) = x^2`.
    pub fn try_from_with_op(expr: Expr, op: &AssignOp) -> ParseResult<Self> {
        match expr {
            Expr::Literal(Literal::Symbol(symbol)) => ParseResult::Ok(AssignTarget::Symbol(symbol)),
            Expr::Call(call) => {
                let call_span = call.span();
                ParseResult::Recoverable(
                    AssignTarget::Symbol(call.name),
                    vec![Error::new(
                        vec![call_span, op.span.clone()],
                        InvalidAssignmentLhs { is_call: true },
                    )]
                )
            },
            expr => ParseResult::Recoverable(
                AssignTarget::Symbol(LitSym {
                    name: String::new(),
                    span: expr.span(),
                }),
                vec![Error::new(
                    vec![expr.span(), op.span.clone()],
                    InvalidAssignmentLhs { is_call: false },
                )]
            ),
        }
    }
}

impl<'source> Parse<'source> for AssignTarget {
    fn std_parse(
        input: &mut Parser<'source>,
        recoverable_errors: &mut Vec<Error>
    ) -> Result<Self, Vec<Error>> {
        let _ = return_if_ok!(input.try_parse::<FuncHeader>().map(AssignTarget::Func).forward_errors(recoverable_errors));
        input.try_parse::<LitSym>().map(AssignTarget::Symbol).forward_errors(recoverable_errors)
    }
}

/// An assignment of a variable or function, such as `x = 1` or `f(x) = x^2`.
#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    /// The target to assign to.
    pub target: AssignTarget,

    /// The expression to assign to the target.
    pub value: Box<Expr>,

    /// The region of the source code that this assignment expression was parsed from.
    pub span: Range<usize>,
}

impl Assign {
    /// Returns the span of the assignment expression.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Returns true if the assignment is to a function, and the function body references itself.
    pub fn is_recursive(&self) -> bool {
        if let AssignTarget::Func(header) = &self.target {
            let is_correct_call = |expr: &Expr| {
                match expr {
                    Expr::Call(call) => call.name.name == header.name.name,
                    _ => false,
                }
            };

            self.value.post_order_iter().any(is_correct_call)
        } else {
            false
        }
    }
}

impl<'source> Parse<'source> for Assign {
    fn std_parse(
        input: &mut Parser<'source>,
        recoverable_errors: &mut Vec<Error>
    ) -> Result<Self, Vec<Error>> {
        let target = input.try_parse::<AssignTarget>().forward_errors(recoverable_errors)?;
        input.try_parse::<AssignOp>().forward_errors(recoverable_errors)?;
        let value = input.try_parse::<Expr>().forward_errors(recoverable_errors)?;

        let span = target.span().start..value.span().end;
        Ok(Self {
            target,
            value: Box::new(value),
            span,
        })
    }
}
