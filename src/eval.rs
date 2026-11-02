use std::collections::HashMap;

use crate::{Expr, MAX_DEPTH, MAX_INPUT_BYTES, MAX_VARIABLES};

/// Caller-owned persistent variable bindings.
///
/// Evaluation checks each looked-up value for finiteness, including values
/// inserted directly by callers. Built-in operation names are not bindings.
pub type Env = HashMap<String, f64>;

/// Evaluate an expression, returning a finite number or a recoverable error.
///
/// Arguments run left-to-right and stop at the first error. Nonempty `+` and
/// `*` fold from their first argument, preserving a single argument's signed
/// zero. Empty `+` and `*` return `0` and `1`, respectively. `-` and `/` require
/// exactly two arguments. Division rejects both positive and negative zero.
///
/// `define` requires a symbol and a right-hand expression. It returns the value
/// and replaces its target only after the right-hand side succeeds. Successful
/// nested definitions are never rolled back, including when a later argument,
/// arithmetic operation, or outer definition fails. Shape/arity validation
/// precedes argument evaluation. The variable-cap check happens after the RHS,
/// so nested definitions cannot bypass it.
///
/// # Errors
///
/// Rejects unknown variables and operations, invalid list heads, empty lists,
/// wrong arity, invalid definition targets, non-finite values/results, zero
/// divisors, nesting beyond [`MAX_DEPTH`], and creation of new names at
/// [`MAX_VARIABLES`]. Manually supplied symbols longer than [`MAX_INPUT_BYTES`]
/// bytes are rejected before lookup, cloning, or diagnostic interpolation.
/// Depth checks apply to caller-constructed trees as well as parsed input.
pub fn eval(expression: &Expr, env: &mut Env) -> Result<f64, String> {
    eval_at(expression, env, 0)
}

fn eval_at(expression: &Expr, env: &mut Env, depth: usize) -> Result<f64, String> {
    match expression {
        Expr::Number(number) => finite(*number),
        Expr::Symbol(name) => {
            validate_symbol(name)?;
            let value = env
                .get(name)
                .ok_or_else(|| format!("unknown variable: {name}"))?;
            finite(*value)
        }
        Expr::List(expressions) => {
            if depth >= MAX_DEPTH {
                return Err("maximum nesting depth exceeded".to_owned());
            }
            eval_list(expressions, env, depth + 1)
        }
    }
}

fn eval_list(expressions: &[Expr], env: &mut Env, depth: usize) -> Result<f64, String> {
    let Some((head, args)) = expressions.split_first() else {
        return Err("cannot evaluate empty list".to_owned());
    };
    let Expr::Symbol(operation) = head else {
        return Err("operation head must be a symbol".to_owned());
    };
    validate_symbol(operation)?;
    match operation.as_str() {
        "+" | "*" => {
            let Some((first, rest)) = args.split_first() else {
                return Ok(if operation == "+" { 0.0 } else { 1.0 });
            };
            let mut result = eval_at(first, env, depth)?;
            for argument in rest {
                let value = eval_at(argument, env, depth)?;
                result = finite(if operation == "+" {
                    result + value
                } else {
                    result * value
                })?;
            }
            Ok(result)
        }
        "-" | "/" => {
            let [left, right] = args else {
                return Err(format!("'{operation}' expects exactly 2 arguments"));
            };
            let left = eval_at(left, env, depth)?;
            let right = eval_at(right, env, depth)?;
            if operation == "-" {
                finite(left - right)
            } else if right == 0.0 {
                Err("division by zero".to_owned())
            } else {
                finite(left / right)
            }
        }
        "define" => {
            let [target, rhs] = args else {
                return Err("'define' expects exactly 2 arguments".to_owned());
            };
            let Expr::Symbol(name) = target else {
                return Err("'define' target must be a symbol".to_owned());
            };
            validate_symbol(name)?;
            let value = eval_at(rhs, env, depth)?;
            if !env.contains_key(name) && env.len() >= MAX_VARIABLES {
                return Err("maximum variable count exceeded".to_owned());
            }
            *env.entry(name.clone()).or_insert(value) = value;
            Ok(value)
        }
        _ => Err(format!("unknown operation: {operation}")),
    }
}

fn validate_symbol(symbol: &str) -> Result<(), String> {
    if symbol.len() > MAX_INPUT_BYTES {
        Err("symbol exceeds byte limit".to_owned())
    } else {
        Ok(())
    }
}

fn finite(value: f64) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("number must be finite".to_owned())
    }
}
