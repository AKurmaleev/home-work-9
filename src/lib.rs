#![forbid(unsafe_code)]

//! A small, finite-number Lisp interpreter with a caller-owned environment.
//!
//! Supported forms are `+`, `-`, `*`, `/`, and `define`. Addition and
//! multiplication accept any number of arguments; subtraction and division
//! require exactly two. Empty addition returns zero; empty multiplication
//! returns one. Nonempty arithmetic is evaluated left-to-right, stopping at
//! the first error, with ordinary `f64` rounding after each operation.
//!
//! Signed zero and subnormal numbers are valid. Either sign of a zero divisor
//! is rejected. Non-finite literals, looked-up values, and intermediate results
//! are rejected. Definitions persist in the supplied environment. Evaluation is
//! not transactional: completed nested definitions survive a later error, even
//! in an unsuccessful outer definition.
//!
//! ```
//! use mini_lisp::{run_line, Env};
//!
//! fn main() -> Result<(), String> {
//!     let mut env = Env::new();
//!     assert_eq!(run_line("(define x 10)", &mut env)?, 10.0);
//!     assert_eq!(run_line("(+ x (* 2 3))", &mut env)?, 16.0);
//!     assert_eq!(run_line("unknown", &mut env).unwrap_err(),
//!                "unknown variable: unknown");
//!     Ok(())
//! }
//! ```

mod eval;
/// Interactive input and output, separate from language evaluation.
pub mod repl;
mod syntax;

pub use eval::{Env, eval};
pub use syntax::{Expr, parse, parse_expr, parse_list, tokenize};

/// Maximum UTF-8 input size, in bytes, accepted by [`run_line`].
///
/// Parsing also limits the aggregate token bytes and token count to this value
/// before allocating an AST. Standalone [`tokenize`] does not enforce limits.
pub const MAX_INPUT_BYTES: usize = 16_384;

/// Maximum number of nested lists accepted by parsing and evaluation.
///
/// A standalone atom has depth zero; its enclosing list adds one. Evaluation
/// enforces the same limit for caller-constructed expressions.
pub const MAX_DEPTH: usize = 64;

/// Maximum number of variable names created through evaluation.
///
/// Existing names may still be reassigned when the environment is full.
/// Callers can populate [`Env`] directly; evaluation does not shrink it.
pub const MAX_VARIABLES: usize = 1_024;

/// Tokenize, parse, and evaluate exactly one expression using a persistent env.
///
/// # Errors
///
/// Returns a recoverable diagnostic for oversized input, invalid syntax,
/// unsupported operations, invalid arguments, unknown variables, non-finite
/// numbers, division by zero, or exceeded resource limits. Input bytes are
/// bounded before tokenization. Parsing completes before any mutation; once
/// evaluation starts, successful nested definitions are not rolled back.
pub fn run_line(input: &str, env: &mut Env) -> Result<f64, String> {
    if input.len() > MAX_INPUT_BYTES {
        return Err("input exceeds byte limit".to_owned());
    }
    let expression = parse(&tokenize(input))?;
    eval(&expression, env)
}
