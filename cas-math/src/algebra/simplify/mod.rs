//! Module to simplify expressions.
//!
//! This module provides the [`simplify`] function, which attempts to reduce the complexity of an
//! expression. It does this by repeatedly applying rewriting rules to the expression in multiple
//! passes, until no more rules apply.
//!
//! Complexity is an informal, arbitrary metric that is used to determine whether one expression is
//! simpler than another. The default complexity heuristic used is [`default_complexity`] (click
//! for more information). However, this can be overridden by providing a custom complexity
//! function to the ``simplify_with`` function.

pub mod rules;
pub mod step;

use crate::step::StepCollector;
use cas_eval::consts::float;
use step::Step;
use super::expr::{Expr, Primary};

/// The default complexity heuristic function.
///
/// This function computes complexity using these simple rules:
///
/// - `complexity(number) = abs(number)`
/// - `complexity(symbol) = length(symbol)`
/// - `complexity(call) = length(name) + length(args)`
/// - `complexity(add) = 3 + sum(complexity(terms))`
/// - `complexity(mul) = 2 + sum(complexity(factors))`
/// - `complexity(exp) = 1 + complexity(lhs) + complexity(rhs)`
pub fn default_complexity(expr: &Expr) -> usize {
    expr.post_order_iter()
        .map(|expr| match expr {
            Expr::Primary(primary) => {
                match primary {
                    Primary::Number(num) => float(num.abs_ref())
                        .to_integer().unwrap()
                        .to_usize().unwrap(),
                    Primary::Symbol(sym) => sym.len(),
                    Primary::Call(name, args) => name.len() + args.len(),
                }
            },
            Expr::Add(terms) => 3 + terms.len(),
            Expr::Mul(factors) => 2 + factors.len(),
            Expr::Exp(_, _) => 1,
        })
        .sum()
}

/// Base implementation of the simplification algorithm.
fn inner_simplify_with<F>(
    expr: &Expr,
    complexity: F,
    step_collector: &mut dyn StepCollector<Step>,
) -> (Expr, bool)
where
    F: Copy + Fn(&Expr) -> usize,
{
    let mut expr = expr.clone();
    let mut changed_at_least_once = false;

    loop {
        // TODO: use complexity
        let mut current_complexity = complexity(&expr);
        let mut changed_in_this_pass = false;

        // try to simplify this expression using all rules
        if let Some(new_expr) = rules::all(&expr, step_collector) {
            expr = new_expr;
            changed_in_this_pass = true;
            changed_at_least_once = true;
            continue;
        }

        // then begin recursing into the expression's children
        match expr {
            Expr::Primary(primary) => return (Expr::Primary(primary), changed_at_least_once),
            Expr::Add(ref terms) => {
                let mut output = Expr::Add(Vec::new());
                for term in terms {
                    let result = inner_simplify_with(term, complexity, step_collector);
                    output += result.0;

                    // use |= instead of = to not reset these variables to false if already true
                    changed_in_this_pass |= result.1;
                    changed_at_least_once |= result.1;
                }
                expr = output;
            },
            Expr::Mul(ref mut factors) => {
                for factor in factors.iter_mut() {
                    let result = inner_simplify_with(factor, complexity, step_collector);
                    *factor = result.0;
                    changed_in_this_pass |= result.1;
                    changed_at_least_once |= result.1;
                }
            },
            Expr::Exp(ref mut lhs, ref mut rhs) => {
                let result_l = inner_simplify_with(lhs, complexity, step_collector);
                let result_r = inner_simplify_with(rhs, complexity, step_collector);

                *lhs = Box::new(result_l.0);
                *rhs = Box::new(result_r.0);
                changed_in_this_pass |= result_l.1 || result_r.1;
                changed_at_least_once |= result_l.1 || result_r.1;
            },
        }

        if !changed_in_this_pass {
            break;
        }
    }

    (expr, changed_at_least_once)
}

/// Simplify the given expression, using the default complexity heuristic function.
pub fn simplify(expr: &Expr) -> Expr {
    inner_simplify_with(expr, default_complexity, &mut ()).0
}

/// Simplify the given expression, using the given complexity heuristic function.
///
/// The complexity heuristic function should return a number that represents the complexity of the
/// given expression. The lower the number, the simpler the expression.
pub fn simplify_with<F>(expr: &Expr, complexity: F) -> Expr
where
    F: Copy + Fn(&Expr) -> usize,
{
    inner_simplify_with(expr, complexity, &mut ()).0
}

/// Simplify the given expression, using the default complexity heuristic function. The steps taken
/// by the simplifier will also be collected and returned. This is useful for debugging, and also
/// for displaying the steps taken to the user.
pub fn simplify_with_steps(expr: &Expr) -> (Expr, Vec<Step>) {
    let mut steps = Vec::new();
    let expr = inner_simplify_with(expr, default_complexity, &mut steps).0;
    (expr, steps)
}

#[cfg(test)]
mod tests {
    use cas_parser::parser::{ast::expr::Expr as AstExpr, Parser};
    use pretty_assertions::assert_eq;
    use super::*;

    #[test]
    fn add_rules() {
        // also tests multiply_zero
        let input = String::from("0+0*(3x+5b^2i)+0+(3a)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Mul(vec![
            Expr::Primary(Primary::Symbol(String::from("a"))),
            Expr::Primary(Primary::Number(float(3))),
        ]));
    }

    #[test]
    fn combine_like_terms() {
        let input = String::from("-9(6m-3) + 6(1+4m)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Add(vec![
            Expr::Mul(vec![
                Expr::Primary(Primary::Symbol(String::from("m"))),
                Expr::Primary(Primary::Number(float(-30))),
            ]),
            Expr::Primary(Primary::Number(float(33))),
        ]));
    }

    #[test]
    fn multiply_rules() {
        let input = String::from("0*(3x+5b^2i)*1*(3a)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Number(float(0))));
    }

    #[test]
    fn multiply_rules_2() {
        // also tests add_zero
        let input = String::from("1*3*1*1*1*(1+(x^2+5x+6)*0)*1*1");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Number(float(3))));
    }

    #[test]
    fn combine_like_factors() {
        let input = String::from("a * b * a^3 * c^2 * d^2 * a^2 * b^4 * d^2");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Mul(vec![
            Expr::Exp(
                Box::new(Expr::Primary(Primary::Symbol("d".to_string()))),
                Box::new(Expr::Primary(Primary::Number(float(4)))),
            ),
            Expr::Exp(
                Box::new(Expr::Primary(Primary::Symbol("b".to_string()))),
                Box::new(Expr::Primary(Primary::Number(float(5)))),
            ),
            Expr::Exp(
                Box::new(Expr::Primary(Primary::Symbol("a".to_string()))),
                Box::new(Expr::Primary(Primary::Number(float(6)))),
            ),
            Expr::Exp(
                Box::new(Expr::Primary(Primary::Symbol("c".to_string()))),
                Box::new(Expr::Primary(Primary::Number(float(2)))),
            ),
        ]));
    }

    #[test]
    fn combine_like_factors_strict_eq() {
        let input = String::from("(a + 1 + b) * (b + a) * (b + a + 1) * (a + b)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Mul(vec![
            Expr::Exp(
                Box::new(Expr::Add(vec![
                    Expr::Primary(Primary::Symbol("a".to_string())),
                    Expr::Primary(Primary::Symbol("b".to_string())),
                    Expr::Primary(Primary::Number(float(1))),
                ])),
                Box::new(Expr::Primary(Primary::Number(float(2)))),
            ),
            Expr::Exp(
                Box::new(Expr::Add(vec![
                    Expr::Primary(Primary::Symbol("a".to_string())),
                    Expr::Primary(Primary::Symbol("b".to_string())),
                ])),
                Box::new(Expr::Primary(Primary::Number(float(2)))),
            ),
        ]));
    }

    #[test]
    fn simple_combine_like_factors() {
        let input = String::from("(a+b)/(a+b)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Number(float(1))));
    }

    #[test]
    fn distribute() {
        // 1/x * (y+2x) = y/x + 2
        let input = String::from("1/x * (y+2x)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let (simplified_expr, steps) = simplify_with_steps(&math_expr);
        assert_eq!(simplified_expr, Expr::Add(vec![
            Expr::Mul(vec![
                Expr::Primary(Primary::Symbol("y".to_string())),
                Expr::Exp(
                    Box::new(Expr::Primary(Primary::Symbol("x".to_string()))),
                    Box::new(Expr::Primary(Primary::Number(float(-1)))),
                ),
            ]),
            Expr::Primary(Primary::Number(float(2))),
        ]));
        assert!(steps.contains(&Step::DistributiveProperty));
    }

    #[test]
    fn distribute_2() {
        // x^2 * (1 + x + y/x^2) = x^2 + x^3 + y
        let input = String::from("x^2 * (1 + x + y/x^2)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let (simplified_expr, steps) = simplify_with_steps(&math_expr);
        assert_eq!(simplified_expr, Expr::Add(vec![
            Expr::Exp(
                Box::new(Expr::Primary(Primary::Symbol("x".to_string()))),
                Box::new(Expr::Primary(Primary::Number(float(2)))),
            ),
            Expr::Exp(
                Box::new(Expr::Primary(Primary::Symbol("x".to_string()))),
                Box::new(Expr::Primary(Primary::Number(float(3)))),
            ),
            Expr::Primary(Primary::Symbol("y".to_string())),
        ]));
        assert!(steps.contains(&Step::DistributiveProperty));
    }

    #[test]
    fn power_rules() {
        let input = String::from("(1^0)^(3x+5b^2i)^1^(3a)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Number(float(1))));
    }

    #[test]
    fn power_rules_2() {
        let input = String::from("(0^1)^0");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Number(float(1))));
    }

    #[test]
    fn power_rules_3a() {
        let input = String::from("x^3 * x^-2");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Symbol("x".to_string())));
    }

    #[test]
    fn power_rules_3b() {
        let input = String::from("x^3 / x^2");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let simplified_expr = simplify(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Symbol("x".to_string())));
    }

    #[test]
    fn power_rule_steps() {
        let input = String::from("(1^0)^(3x+5b^2i)^1^(3a)");
        let expr = Parser::new(&input).try_parse_full::<AstExpr>().unwrap();
        let math_expr = Expr::from(expr);
        let (simplified_expr, steps) = simplify_with_steps(&math_expr);
        assert_eq!(simplified_expr, Expr::Primary(Primary::Number(float(1))));
        assert_eq!(steps, vec![
            Step::PowerPower,
            Step::PowerOneLeft,
        ]);
    }
}
