//! Built-in functions for CalcScript.

mod error;

use cas_attrs::args;
use error::BuiltinError;
use super::{
    error::kind::{MissingArgument, TooManyArguments, TypeMismatch},
    value::Value::{self, *},
};

/// Generates builtin implementations for simple one-argument functions that take a number.
macro_rules! generate_number_builtin {
    ($($name:ident)+) => {
        $(
            #[args(n: Number)]
            pub fn $name(args: &[Value]) -> Result<Value, BuiltinError> {
                Ok(Number(n.$name()))
            }
        )*
    };
}

generate_number_builtin!(
    // trigonometric functions
    sin cos tan asin acos atan
    sinh cosh tanh asinh acosh atanh

    // exponential and logarithmic functions
    exp ln

    abs
);

#[args(y: Number, x: Number)]
pub fn atan2(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(y.atan2(*x)))
}

#[args(n: Number)]
pub fn csc(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(1.0 / n.sin()))
}

#[args(n: Number)]
pub fn sec(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(1.0 / n.cos()))
}

#[args(n: Number)]
pub fn cot(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(1.0 / n.tan()))
}

#[args(n: Number)]
pub fn acsc(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number((1.0 / *n).asin()))
}

#[args(n: Number)]
pub fn asec(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number((1.0 / *n).acos()))
}

#[args(n: Number)]
pub fn acot(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number((1.0 / *n).atan()))
}

#[args(n: Number)]
pub fn csch(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(1.0 / n.sinh()))
}

#[args(n: Number)]
pub fn sech(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(1.0 / n.cosh()))
}

#[args(n: Number)]
pub fn coth(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(1.0 / n.tanh()))
}

// TODO: acsch, asech, acoth

#[args(n: Number)]
pub fn dtr(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(n.to_radians()))
}

#[args(n: Number)]
pub fn rtd(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(n.to_degrees()))
}

// TODO: circle

#[args(a: Number, b: Number)]
pub fn scientific(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(*a * 10.0_f64.powf(*b)))
}

#[args(x: Number, y: Number = 10.0)]
pub fn log(args: &[Value]) -> Result<Value, BuiltinError> {
    Ok(Number(x.log(*y)))
}

/// Returns the builtin function with the given name.
pub fn get_builtin(name: &str) -> Option<fn(&[Value]) -> Result<Value, BuiltinError>> {
    macro_rules! match_builtin {
        ($($name:ident)+) => {
            match name {
                $(
                    stringify!($name) => Some($name),
                )*
                _ => None,
            }
        };
    }

    match_builtin!(
        // trigonometric functions
        sin cos tan asin acos atan atan2
        csc sec cot acsc asec acot
        sinh cosh tanh asinh acosh atanh
        csch sech coth

        // conversion functions
        dtr rtd

        // exponential and logarithmic functions
        exp scientific log ln

        abs
    )
}
